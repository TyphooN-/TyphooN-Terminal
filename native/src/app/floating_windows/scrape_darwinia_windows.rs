use super::*;

impl TyphooNApp {
    pub(super) fn render_scrape_darwinia_windows(&mut self, ctx: &egui::Context) {
        // ── SwapHarvest Window ──
        if self.show_swap_harvest {
            let mut swap_pending_action = SymbolAction::None;
            egui::Window::new("SwapHarvest — Positive Swap Scanner")
                        .open(&mut self.show_swap_harvest)
                        .resizable(true).default_size([900.0, 600.0])
        .max_size([900.0, 640.0])
                        .show(ctx, |ui| {
                            if let Some(ref result) = self.swap_harvest_results {
                                // Summary bar
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format!("{} symbols with positive swap", result.entries.len())).strong());
                                    ui.separator();
                                    ui.label(egui::RichText::new(format!("Long: {}", result.long_count)).color(ACCENT));
                                    ui.separator();
                                    ui.label(egui::RichText::new(format!("Short: {}", result.short_count)).color(egui::Color32::from_rgb(255, 100, 100)));
                                    ui.separator();
                                    ui.label(egui::RichText::new(format!("Both: {}", result.both_count)).color(egui::Color32::from_rgb(100, 180, 255)));
                                    ui.separator();
                                    ui.label(egui::RichText::new(format!("Scanned: {}", result.total_scanned)).color(AXIS_TEXT));
                                });
                                ui.add_space(4.0);

                                // Filters
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Filter:").color(AXIS_TEXT).small());
                                    ui.add(egui::TextEdit::singleline(&mut self.swap_harvest_filter).desired_width(120.0).hint_text("symbol...").font(egui::TextStyle::Monospace));
                                    ui.separator();
                                    ui.label(egui::RichText::new("Direction:").color(AXIS_TEXT).small());
                                    if ui.selectable_label(self.swap_harvest_dir_filter.is_empty(), "All").clicked() {
                                        self.swap_harvest_dir_filter.clear();
                                    }
                                    if ui.selectable_label(self.swap_harvest_dir_filter == "LONG", "Long").clicked() {
                                        self.swap_harvest_dir_filter = "LONG".into();
                                    }
                                    if ui.selectable_label(self.swap_harvest_dir_filter == "SHORT", "Short").clicked() {
                                        self.swap_harvest_dir_filter = "SHORT".into();
                                    }
                                    if ui.selectable_label(self.swap_harvest_dir_filter == "BOTH", "Both").clicked() {
                                        self.swap_harvest_dir_filter = "BOTH".into();
                                    }
                                    // Export button
                                    ui.separator();
                                    if ui.add(egui::Button::new(egui::RichText::new("Export CSV").color(BTN_GREEN_TEXT).small()).fill(BTN_GREEN)).clicked() {
                                        if let Some(ref cache) = self.cache {
                                            if let Some(conn) = cache.try_connection() {
                                                let mut out = dirs_home(); out.push("export");
                                                let _ = std::fs::create_dir_all(&out);
                                                let path = out.join(format!("SwapHarvest-{}.csv", chrono::Utc::now().format("%Y-%m-%d")));
                                                let mut csv = String::from("Symbol;Direction;SwapLong;SwapShort;Spread;VolumeMin;MarginInitial;Sector;Industry;Description\n");
                                                for e in &result.entries {
                                                    csv.push_str(&format!("{};{};{:.4};{:.4};{};{};{:.0};{};{};{}\n",
                                                        e.symbol, e.direction, e.swap_long, e.swap_short, e.spread, e.volume_min, e.margin_initial, e.sector, e.industry, e.description));
                                                }
                                                match std::fs::write(&path, &csv) {
                                                    Ok(_) => self.log.push_back(LogEntry::info(format!("SwapHarvest CSV exported: {}", path.display()))),
                                                    Err(e) => self.log.push_back(LogEntry::err(format!("Export failed: {}", e))),
                                                }
                                                drop(conn);
                                            }
                                        }
                                    }
                                });
                                ui.separator();

                                // Table
                                let filter_upper = self.swap_harvest_filter.to_uppercase();
                                let dir_filter = self.swap_harvest_dir_filter.clone();
                                egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                                    egui::Grid::new("swap_harvest_grid").striped(true).num_columns(9).min_col_width(50.0).show(ui, |ui| {
                                        // Header
                                        ui.strong("Symbol");
                                        ui.strong("Direction");
                                        ui.strong("Swap Long");
                                        ui.strong("Swap Short");
                                        ui.strong("Best");
                                        ui.strong("Spread");
                                        ui.strong("Min Lot");
                                        ui.strong("Margin");
                                        ui.strong("Description");
                                        ui.end_row();

                                        for e in &result.entries {
                                            // Apply filters
                                            if !filter_upper.is_empty() && !e.symbol.to_uppercase().contains(&filter_upper) && !e.description.to_uppercase().contains(&filter_upper) {
                                                continue;
                                            }
                                            if !dir_filter.is_empty() && e.direction != dir_filter {
                                                continue;
                                            }

                                            ui.horizontal(|ui| {
                                                let (_, sw_action) = symbol_label_with_menu(
                                                    ui,
                                                    &e.symbol,
                                                    egui::RichText::new(&e.symbol).monospace().strong(),
                                                );
                                                if !matches!(sw_action, SymbolAction::None) {
                                                    swap_pending_action = sw_action;
                                                }
                                                if ui
                                                    .small_button(egui::RichText::new("+").small())
                                                    .on_hover_text("Open new chart")
                                                    .clicked()
                                                {
                                                    swap_pending_action = SymbolAction::OpenChart(e.symbol.clone());
                                                }
                                            });
                                            let dir_color = match e.direction.as_str() {
                                                "LONG" => ACCENT,
                                                "SHORT" => egui::Color32::from_rgb(255, 100, 100),
                                                _ => egui::Color32::from_rgb(100, 180, 255),
                                            };
                                            ui.label(egui::RichText::new(&e.direction).color(dir_color).small());
                                            // Color swap values: green if positive, red if negative
                                            let swap_l_color = if e.swap_long > 0.0 { ACCENT } else { egui::Color32::from_rgb(255, 100, 100) };
                                            let swap_s_color = if e.swap_short > 0.0 { ACCENT } else { egui::Color32::from_rgb(255, 100, 100) };
                                            ui.label(egui::RichText::new(format!("{:.4}", e.swap_long)).color(swap_l_color).small().monospace());
                                            ui.label(egui::RichText::new(format!("{:.4}", e.swap_short)).color(swap_s_color).small().monospace());
                                            ui.label(egui::RichText::new(format!("{:.4}", e.best_swap)).color(egui::Color32::from_rgb(255, 215, 0)).small().monospace());
                                            ui.label(egui::RichText::new(format!("{}", e.spread)).color(AXIS_TEXT).small().monospace());
                                            ui.label(egui::RichText::new(format!("{}", e.volume_min)).color(AXIS_TEXT).small().monospace());
                                            ui.label(egui::RichText::new(format!("{:.0}", e.margin_initial)).color(AXIS_TEXT).small().monospace());
                                            ui.label(egui::RichText::new(&e.description).color(AXIS_TEXT).small());
                                            ui.end_row();
                                        }
                                    });
                                });
                            } else {
                                ui.label("No data — run SWAPHARVEST first.");
                            }
                        });
            self.apply_symbol_action(swap_pending_action);
        }

        // ── DarwinexRadar Window ──
        if self.show_darwinex_radar {
            let mut radar_pending_action = SymbolAction::None;
            egui::Window::new("Darwinex Radar — All MT5 Symbols")
                .open(&mut self.show_darwinex_radar)
                .resizable(true)
                .default_size([950.0, 600.0])
                .max_size([950.0, 640.0])
                .show(ctx, |ui| {
                    let data = &self.darwinex_radar_data;
                    // Summary
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{} symbols loaded", data.len())).strong(),
                        );
                        ui.separator();
                        let sectors: std::collections::HashSet<&str> = data
                            .iter()
                            .map(|d| d.1.as_str())
                            .filter(|s| !s.is_empty())
                            .collect();
                        ui.label(
                            egui::RichText::new(format!("{} sectors", sectors.len()))
                                .color(AXIS_TEXT),
                        );
                        ui.separator();
                        let tradeable = data.iter().filter(|d| d.3 > 0).count();
                        ui.label(
                            egui::RichText::new(format!("{} tradeable", tradeable)).color(ACCENT),
                        );
                        // Export button
                        ui.separator();
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Export CSVs")
                                        .color(BTN_GREEN_TEXT)
                                        .small(),
                                )
                                .fill(BTN_GREEN),
                            )
                            .clicked()
                        {
                            if let Some(ref cache) = self.cache {
                                if let Some(conn) = cache.try_connection() {
                                    let mut out = dirs_home();
                                    out.push("export");
                                    let _ = std::fs::create_dir_all(&out);
                                    match darwin::export_radar_txt(
                                        &conn,
                                        &conn,
                                        &out.display().to_string(),
                                    ) {
                                        Ok(msg) => self.log.push_back(LogEntry::info(format!(
                                            "Radar exported: {}",
                                            msg
                                        ))),
                                        Err(e) => self.log.push_back(LogEntry::err(format!(
                                            "Export failed: {}",
                                            e
                                        ))),
                                    }
                                }
                            }
                        }
                    });
                    ui.separator();

                    // Sector tabs
                    let mut categories: Vec<(&str, usize)> = Vec::new();
                    let mut currency_count = 0;
                    let mut index_count = 0;
                    let mut commodity_count = 0;
                    let mut tech_count = 0;
                    let mut healthcare_count = 0;
                    let mut other_count = 0;
                    for d in data {
                        match d.1.as_str() {
                            "Currency" => currency_count += 1,
                            "Indexes" => index_count += 1,
                            "Commodities" => commodity_count += 1,
                            "Technology" => tech_count += 1,
                            "Healthcare" => healthcare_count += 1,
                            _ => other_count += 1,
                        }
                    }
                    categories.push(("All", data.len()));
                    if currency_count > 0 {
                        categories.push(("Currency", currency_count));
                    }
                    if index_count > 0 {
                        categories.push(("Indexes", index_count));
                    }
                    if commodity_count > 0 {
                        categories.push(("Commodities", commodity_count));
                    }
                    if tech_count > 0 {
                        categories.push(("Technology", tech_count));
                    }
                    if healthcare_count > 0 {
                        categories.push(("Healthcare", healthcare_count));
                    }
                    if other_count > 0 {
                        categories.push(("Other", other_count));
                    }

                    // Changelog section
                    if !self.darwinex_radar_changelog.is_empty() {
                        ui.collapsing(
                            egui::RichText::new(format!(
                                "Changelog ({} changes)",
                                self.darwinex_radar_changelog.len()
                            ))
                            .strong(),
                            |ui| {
                                egui::Grid::new("radar_changelog_grid")
                                    .striped(true)
                                    .num_columns(3)
                                    .min_col_width(60.0)
                                    .show(ui, |ui| {
                                        ui.strong("Symbol");
                                        ui.strong("Type");
                                        ui.strong("Detail");
                                        ui.end_row();
                                        for c in &self.darwinex_radar_changelog {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(&c.symbol)
                                                        .monospace()
                                                        .strong(),
                                                );
                                                if ui
                                                    .small_button(egui::RichText::new("+").small())
                                                    .on_hover_text("Open new chart")
                                                    .clicked()
                                                {
                                                    radar_pending_action =
                                                        SymbolAction::OpenChart(c.symbol.clone());
                                                }
                                            });
                                            let (type_color, type_label) = match c
                                                .change_type
                                                .as_str()
                                            {
                                                "NEW" => (ACCENT, "NEW"),
                                                "REMOVED" => (
                                                    egui::Color32::from_rgb(255, 100, 100),
                                                    "REMOVED",
                                                ),
                                                "MODE_CHANGED" => (egui::Color32::YELLOW, "MODE"),
                                                "SWAP_CHANGED" => {
                                                    (egui::Color32::from_rgb(100, 180, 255), "SWAP")
                                                }
                                                "SPREAD_CHANGED" => (AXIS_TEXT, "SPREAD"),
                                                _ => (AXIS_TEXT, "OTHER"),
                                            };
                                            ui.label(
                                                egui::RichText::new(type_label)
                                                    .color(type_color)
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&c.detail)
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            },
                        );
                        ui.separator();
                    } else {
                        ui.label(
                            egui::RichText::new(
                                "No changes since last snapshot (run again tomorrow to see diffs)",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        ui.separator();
                    }

                    // Table
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            egui::Grid::new("radar_grid")
                                .striped(true)
                                .num_columns(9)
                                .min_col_width(50.0)
                                .show(ui, |ui| {
                                    ui.strong("Symbol");
                                    ui.strong("Sector");
                                    ui.strong("Industry");
                                    ui.strong("Mode");
                                    ui.strong("Swap Long");
                                    ui.strong("Swap Short");
                                    ui.strong("Min Lot");
                                    ui.strong("Margin");
                                    ui.strong("Description");
                                    ui.end_row();

                                    for d in data {
                                        let (
                                            ref sym,
                                            ref sector,
                                            ref industry,
                                            mode,
                                            swap_l,
                                            swap_s,
                                            vol_min,
                                            margin,
                                            ref desc,
                                        ) = *d;
                                        let mode_text = match mode {
                                            0 => "Disabled",
                                            4 => "Full",
                                            _ => "Partial",
                                        };
                                        let mode_color = if mode >= 4 {
                                            ACCENT
                                        } else if mode > 0 {
                                            egui::Color32::YELLOW
                                        } else {
                                            egui::Color32::from_rgb(255, 100, 100)
                                        };
                                        ui.horizontal(|ui| {
                                            let (_, rd_action) = symbol_label_with_menu(
                                                ui,
                                                sym,
                                                egui::RichText::new(sym).monospace().strong(),
                                            );
                                            if !matches!(rd_action, SymbolAction::None) {
                                                radar_pending_action = rd_action;
                                            }
                                            if ui
                                                .small_button(egui::RichText::new("+").small())
                                                .on_hover_text("Open new chart")
                                                .clicked()
                                            {
                                                radar_pending_action =
                                                    SymbolAction::OpenChart(sym.clone());
                                            }
                                        });
                                        ui.label(
                                            egui::RichText::new(sector).color(AXIS_TEXT).small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(industry).color(AXIS_TEXT).small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(mode_text)
                                                .color(mode_color)
                                                .small(),
                                        );
                                        let sl_color = if swap_l > 0.0 {
                                            ACCENT
                                        } else if swap_l < 0.0 {
                                            egui::Color32::from_rgb(255, 100, 100)
                                        } else {
                                            AXIS_TEXT
                                        };
                                        let ss_color = if swap_s > 0.0 {
                                            ACCENT
                                        } else if swap_s < 0.0 {
                                            egui::Color32::from_rgb(255, 100, 100)
                                        } else {
                                            AXIS_TEXT
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("{:.4}", swap_l))
                                                .color(sl_color)
                                                .small()
                                                .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.4}", swap_s))
                                                .color(ss_color)
                                                .small()
                                                .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{}", vol_min))
                                                .color(AXIS_TEXT)
                                                .small()
                                                .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}", margin))
                                                .color(AXIS_TEXT)
                                                .small()
                                                .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(desc).color(AXIS_TEXT).small(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                });
            self.apply_symbol_action(radar_pending_action);
        }

        // ── Scrape Status Dashboard ──
        if self.show_scrape_status {
            let scrape_status_sec_scope_label = self.broker_scope_label();
            let mut scrape_status_sec_clicked = false;
            egui::Window::new("Scrape Status Dashboard")
                .open(&mut self.show_scrape_status)
                .resizable(true)
                .default_size([700.0, 400.0])
                .show(ctx, |ui| {
                    let indicator = |running: bool, msg: &str| -> (egui::Color32, &str) {
                        if running {
                            (egui::Color32::YELLOW, "\u{25B6}")
                        }
                        // ▶ running
                        else if msg.is_empty() {
                            (AXIS_TEXT, "\u{25CB}")
                        }
                        // ○ idle
                        else if msg.contains("FAIL")
                            || msg.contains("error")
                            || msg.contains("failed")
                        {
                            (egui::Color32::from_rgb(255, 100, 100), "\u{25CF}")
                        }
                        // ● error
                        else {
                            (ACCENT, "\u{2713}")
                        } // ✓ done
                    };

                    egui::Grid::new("scrape_status_grid")
                        .striped(true)
                        .num_columns(5)
                        .min_col_width(80.0)
                        .show(ui, |ui| {
                            ui.strong("Feature");
                            ui.strong("Status");
                            ui.strong("Progress");
                            ui.strong("Details");
                            ui.strong("Action");
                            ui.end_row();

                            // ── Fundamentals ──
                            let (fund_color, fund_icon) =
                                indicator(self.scrape_fund_running, &self.scrape_fund_last_msg);
                            ui.label(egui::RichText::new("Fundamentals").strong());
                            ui.label(
                                egui::RichText::new(if self.scrape_fund_running {
                                    format!("{} Running", fund_icon)
                                } else if self.scrape_fund_last_msg.is_empty() {
                                    format!("{} Idle", fund_icon)
                                } else {
                                    format!("{} Done", fund_icon)
                                })
                                .color(fund_color),
                            );
                            if self.scrape_fund_total > 0 {
                                let done = self.scrape_fund_ok
                                    + self.scrape_fund_fail
                                    + self.scrape_fund_skipped;
                                let pct = (done as f32 / self.scrape_fund_total as f32 * 100.0)
                                    .min(100.0);
                                ui.horizontal(|ui| {
                                    let bar = egui::ProgressBar::new(pct / 100.0)
                                        .desired_width(120.0)
                                        .text(format!(
                                            "{}/{} ({:.0}%)",
                                            done, self.scrape_fund_total, pct
                                        ));
                                    ui.add(bar);
                                });
                            } else {
                                ui.label(egui::RichText::new("—").color(AXIS_TEXT));
                            }
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{} OK", self.scrape_fund_ok))
                                        .color(ACCENT)
                                        .small(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{} fail", self.scrape_fund_fail))
                                        .color(egui::Color32::from_rgb(255, 100, 100))
                                        .small(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} cached",
                                        self.scrape_fund_skipped
                                    ))
                                    .color(AXIS_TEXT)
                                    .small(),
                                );
                            });
                            if !self.scrape_fund_running {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Scrape")
                                                .color(BTN_GREEN_TEXT)
                                                .small(),
                                        )
                                        .fill(BTN_GREEN),
                                    )
                                    .clicked()
                                {
                                    let db_path = cache_db_path();
                                    let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                                        db_path,
                                        use_mt5: self.fund_source_mt5,
                                        use_alpaca: self.fund_source_alpaca,
                                        use_tastytrade: self.fund_source_tastytrade,
                                        use_kraken: self.fund_source_kraken,
                                        kraken_equity_symbols: self
                                            .kraken_equity_universe_symbols
                                            .clone(),
                                        force: false,
                                    });
                                    self.scrape_fund_running = true;
                                    self.scrape_fund_ok = 0;
                                    self.scrape_fund_fail = 0;
                                    self.scrape_fund_skipped = 0;
                                }
                            } else {
                                ui.label(
                                    egui::RichText::new("running...")
                                        .color(egui::Color32::YELLOW)
                                        .small(),
                                );
                            }
                            ui.end_row();

                            // ── SEC Scrape ──
                            let (sec_color, sec_icon) =
                                indicator(self.scrape_sec_running, &self.scrape_sec_last_msg);
                            ui.label(egui::RichText::new("SEC Filings").strong());
                            ui.label(
                                egui::RichText::new(if self.scrape_sec_running {
                                    format!("{} Running", sec_icon)
                                } else if self.scrape_sec_last_msg.is_empty() {
                                    format!("{} Idle", sec_icon)
                                } else {
                                    format!("{} Done", sec_icon)
                                })
                                .color(sec_color),
                            );
                            ui.label(egui::RichText::new("—").color(AXIS_TEXT));
                            ui.label(
                                egui::RichText::new(if self.scrape_sec_last_msg.len() > 60 {
                                    format!("{}...", &self.scrape_sec_last_msg[..60])
                                } else {
                                    self.scrape_sec_last_msg.clone()
                                })
                                .color(AXIS_TEXT)
                                .small(),
                            );
                            if !self.scrape_sec_running {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Scrape")
                                                .color(BTN_GREEN_TEXT)
                                                .small(),
                                        )
                                        .fill(BTN_GREEN),
                                    )
                                    .clicked()
                                {
                                    scrape_status_sec_clicked = true;
                                }
                            } else {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(
                                        egui::RichText::new("running...")
                                            .color(egui::Color32::YELLOW)
                                            .small(),
                                    );
                                });
                            }
                            ui.end_row();

                            // ── DarwinIA FTP Scan ──
                            let (dar_color, dar_icon) =
                                indicator(self.scrape_darwin_running, &self.scrape_darwin_last_msg);
                            ui.label(egui::RichText::new("DarwinIA FTP").strong());
                            ui.label(
                                egui::RichText::new(if self.scrape_darwin_running {
                                    format!("{} Running", dar_icon)
                                } else if self.scrape_darwin_last_msg.is_empty() {
                                    format!("{} Idle", dar_icon)
                                } else {
                                    format!("{} Done", dar_icon)
                                })
                                .color(dar_color),
                            );
                            ui.label(egui::RichText::new("—").color(AXIS_TEXT));
                            ui.label(
                                egui::RichText::new(&self.scrape_darwin_last_msg)
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                            let ftp_available = !self.darwin_ftp_dir.is_empty();
                            if !self.scrape_darwin_running && ftp_available {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Scan")
                                                .color(BTN_GREEN_TEXT)
                                                .small(),
                                        )
                                        .fill(BTN_GREEN),
                                    )
                                    .clicked()
                                {
                                    if self.gpu_darwin.is_some() {
                                        let _ = self.broker_tx.send(BrokerCmd::DarwinGpuScan {
                                            ftp_dir: self.darwin_ftp_dir.clone(),
                                            min_days: 90,
                                        });
                                    } else {
                                        let _ = self.broker_tx.send(BrokerCmd::DarwinFtpScan {
                                            ftp_dir: self.darwin_ftp_dir.clone(),
                                            min_days: 90,
                                        });
                                    }
                                    self.scrape_darwin_running = true;
                                }
                            } else if !ftp_available {
                                ui.label(
                                    egui::RichText::new("No FTP dir").color(AXIS_TEXT).small(),
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new("running...")
                                        .color(egui::Color32::YELLOW)
                                        .small(),
                                );
                            }
                            ui.end_row();
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    // Per-broker scrape buttons
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Scrape by Broker:").small().strong());
                        let can_scrape = !self.scrape_fund_running;
                        if can_scrape {
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new("MT5 Only").small())
                                        .fill(BTN_GREEN),
                                )
                                .clicked()
                            {
                                let db_path = cache_db_path();
                                let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                                    db_path,
                                    use_mt5: true,
                                    use_alpaca: false,
                                    use_tastytrade: false,
                                    use_kraken: false,
                                    kraken_equity_symbols: self
                                        .kraken_equity_universe_symbols
                                        .clone(),
                                    force: false,
                                });
                                self.scrape_fund_running = true;
                                self.scrape_fund_ok = 0;
                                self.scrape_fund_fail = 0;
                                self.scrape_fund_skipped = 0;
                            }
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new("Alpaca Only").small())
                                        .fill(BTN_GREEN),
                                )
                                .clicked()
                            {
                                let db_path = cache_db_path();
                                let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                                    db_path,
                                    use_mt5: false,
                                    use_alpaca: true,
                                    use_tastytrade: false,
                                    use_kraken: false,
                                    kraken_equity_symbols: self
                                        .kraken_equity_universe_symbols
                                        .clone(),
                                    force: false,
                                });
                                self.scrape_fund_running = true;
                                self.scrape_fund_ok = 0;
                                self.scrape_fund_fail = 0;
                                self.scrape_fund_skipped = 0;
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("TastyTrade Only").small(),
                                    )
                                    .fill(BTN_GREEN),
                                )
                                .clicked()
                            {
                                let db_path = cache_db_path();
                                let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                                    db_path,
                                    use_mt5: false,
                                    use_alpaca: false,
                                    use_tastytrade: true,
                                    use_kraken: false,
                                    kraken_equity_symbols: self
                                        .kraken_equity_universe_symbols
                                        .clone(),
                                    force: false,
                                });
                                self.scrape_fund_running = true;
                                self.scrape_fund_ok = 0;
                                self.scrape_fund_fail = 0;
                                self.scrape_fund_skipped = 0;
                            }
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new("Kraken Only").small())
                                        .fill(BTN_GREEN),
                                )
                                .clicked()
                            {
                                let db_path = cache_db_path();
                                let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                                    db_path,
                                    use_mt5: false,
                                    use_alpaca: false,
                                    use_tastytrade: false,
                                    use_kraken: true,
                                    kraken_equity_symbols: self
                                        .kraken_equity_universe_symbols
                                        .clone(),
                                    force: false,
                                });
                                self.scrape_fund_running = true;
                                self.scrape_fund_ok = 0;
                                self.scrape_fund_fail = 0;
                                self.scrape_fund_skipped = 0;
                            }
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new("All Sources").small())
                                        .fill(BTN_GREEN),
                                )
                                .clicked()
                            {
                                let db_path = cache_db_path();
                                let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                                    db_path,
                                    use_mt5: true,
                                    use_alpaca: true,
                                    use_tastytrade: true,
                                    use_kraken: true,
                                    kraken_equity_symbols: self
                                        .kraken_equity_universe_symbols
                                        .clone(),
                                    force: false,
                                });
                                self.scrape_fund_running = true;
                                self.scrape_fund_ok = 0;
                                self.scrape_fund_fail = 0;
                                self.scrape_fund_skipped = 0;
                            }
                        } else {
                            ui.label(
                                egui::RichText::new("(scrape running)")
                                    .color(egui::Color32::YELLOW)
                                    .small(),
                            );
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Source Checkboxes (for main Scrape button):")
                                .small()
                                .color(AXIS_TEXT),
                        );
                        ui.checkbox(&mut self.fund_source_mt5, "MT5");
                        ui.checkbox(&mut self.fund_source_alpaca, "Alpaca");
                        ui.checkbox(&mut self.fund_source_tastytrade, "TastyTrade");
                        ui.checkbox(&mut self.fund_source_kraken, "Kraken");
                    });
                    // Sync broker_scope from checkbox state
                    self.broker_scope = match (
                        self.fund_source_mt5,
                        self.fund_source_alpaca,
                        self.fund_source_tastytrade,
                        self.fund_source_kraken,
                    ) {
                        (false, true, false, false) => EventSource::Alpaca,
                        (true, false, false, false) => EventSource::Darwinex,
                        (false, false, true, false) => EventSource::Tasty,
                        (false, false, false, true) => EventSource::Kraken,
                        _ => EventSource::All,
                    };

                    // Last message
                    if !self.scrape_fund_last_msg.is_empty() {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(format!("Last: {}", self.scrape_fund_last_msg))
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    }
                });
            if scrape_status_sec_clicked {
                let symbols = self.sec_scrape_scope_symbols();
                if symbols.is_empty() {
                    self.scrape_sec_last_msg = format!(
                        "skipped: Scope {} has no symbols",
                        scrape_status_sec_scope_label
                    );
                } else {
                    let symbol_count = symbols.len();
                    let db_path = cache_db_path();
                    let _ = self
                        .broker_tx
                        .send(BrokerCmd::SecScrape { db_path, symbols });
                    self.scrape_sec_running = true;
                    self.scrape_sec_last_msg = format!(
                        "scraping Scope {} ({} symbols)...",
                        scrape_status_sec_scope_label, symbol_count
                    );
                }
            }
        }

        // Fundamentals Viewer
        if self.show_fundamentals {
            let fund_tickers = self.cached_active_symbols.clone();
            // UX7: Pre-fetch sparklines for all tickers in fundamentals window
            let mut fw_sparklines: std::collections::HashMap<String, std::sync::Arc<Vec<f64>>> =
                std::collections::HashMap::new();
            for t in &fund_tickers {
                let closes = self.get_sparkline(t);
                if !closes.is_empty() {
                    fw_sparklines.insert(t.to_uppercase(), closes);
                }
            }
            egui::Window::new("Fundamentals")
                        .open(&mut self.show_fundamentals)
                        .resizable(true)
                        .default_size([520.0, 480.0])
                        .max_size([900.0, 640.0])
                        .show(ctx, |ui| {
                            let tickers = fund_tickers.clone();

                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("Fundamentals: {} active symbols", tickers.len()))
                                        .strong(),
                                );
                                if ui
                                    .add(egui::Button::new("Full Fundamentals Scrape").fill(BTN_GREEN))
                                    .on_hover_text("Scrape fundamentals for the configured full source universe, not just active charts")
                                    .clicked()
                                {
                                    let db_path = cache_db_path();
                                    let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                                        db_path,
                                        use_mt5: self.fund_source_mt5,
                                        use_alpaca: self.fund_source_alpaca,
                                        use_tastytrade: self.fund_source_tastytrade,
                                        use_kraken: self.fund_source_kraken,
                                        kraken_equity_symbols: self.kraken_equity_universe_symbols.clone(),
                                        force: false,
                                    });
                                    self.log.push_back(LogEntry::info(
                                        "Full fundamentals scrape started for configured source universe...",
                                    ));
                                }
                                if tickers.len() > 1
                                    && ui
                                        .add(egui::Button::new("Scrape Active").fill(BTN_BLUE))
                                        .on_hover_text("Refresh fundamentals only for symbols currently active in charts/windows")
                                        .clicked()
                                {
                                    for t in &tickers {
                                        if !t.is_empty() {
                                            let db_path = cache_db_path();
                                            let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrapeOne {
                                                ticker: t.clone(),
                                                db_path,
                                            });
                                        }
                                    }
                                    self.log.push_back(LogEntry::info(format!(
                                        "Scraping fundamentals for {} active symbols...",
                                        tickers.len()
                                    )));
                                }
                            });
                            ui.separator();

                            let active_symbol_set: std::collections::HashSet<&str> =
                                tickers.iter().map(String::as_str).collect();
                            self.fundamentals_hidden_symbols
                                .retain(|symbol| active_symbol_set.contains(symbol.as_str()));

                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new("Visible symbols:")
                                        .color(AXIS_TEXT)
                                        .small(),
                                );
                                if ui.small_button("All").clicked() {
                                    self.fundamentals_hidden_symbols.clear();
                                }
                                if tickers.len() > 1 && ui.small_button("None").clicked() {
                                    self.fundamentals_hidden_symbols = tickers.iter().cloned().collect();
                                }
                                for ticker in &tickers {
                                    let visible = !self.fundamentals_hidden_symbols.contains(ticker);
                                    let response = ui
                                        .selectable_label(visible, egui::RichText::new(ticker).small())
                                        .on_hover_text("Toggle this symbol in the Fundamentals tile view");
                                    if response.clicked() {
                                        if visible {
                                            self.fundamentals_hidden_symbols.insert(ticker.clone());
                                        } else {
                                            self.fundamentals_hidden_symbols.remove(ticker);
                                        }
                                    }
                                }
                            });
                            ui.separator();

                            let visible_tickers: Vec<&String> = tickers
                                .iter()
                                .filter(|ticker| !self.fundamentals_hidden_symbols.contains(*ticker))
                                .collect();
                            if visible_tickers.is_empty() {
                                ui.label(
                                    egui::RichText::new("No symbols selected. Toggle symbols above or click All.")
                                        .color(AXIS_TEXT),
                                );
                            } else {
                                egui::ScrollArea::vertical()
                                    .id_salt("fundamentals_symbol_tiles")
                                    .auto_shrink(false)
                                    .max_height(ui.available_height().max(240.0))
                                    .show(ui, |ui| {
                                        ui.horizontal_wrapped(|ui| {
                                            for ticker in visible_tickers {
                                                ui.group(|ui| {
                                                    ui.set_min_width(300.0);
                                                    ui.set_max_width(340.0);
                                                    let found = self
                                    .bg
                                    .all_fundamentals
                                    .iter()
                                    .find(|f| f.symbol == *ticker)
                                    .cloned();
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("Fundamentals: {}", ticker)).strong(),
                                    );
                                    if ui
                                        .small_button(egui::RichText::new("+").small())
                                        .on_hover_text("Open new chart")
                                        .clicked()
                                    {
                                        self.deferred_symbol_action =
                                            SymbolAction::OpenChart(ticker.clone());
                                    }
                                    // UX7: ticker is already uppercase (from cached_active_symbols).
                                    if let Some(closes) = fw_sparklines.get(ticker.as_str()) {
                                        draw_inline_sparkline(ui, closes, 80.0, 18.0);
                                    }
                                    if ui
                                        .add(egui::Button::new("Scrape / Refresh").fill(BTN_BLUE))
                                        .clicked()
                                        && !ticker.is_empty()
                                    {
                                        let db_path = cache_db_path();
                                        let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrapeOne {
                                            ticker: ticker.clone(),
                                            db_path,
                                        });
                                        self.log.push_back(LogEntry::info(format!(
                                            "Scraping fundamentals for {}...",
                                            ticker
                                        )));
                                    }
                                });
                                ui.separator();
                                                    if let Some(f) = found {
                                                        // Company info
                                            ui.label(
                                                egui::RichText::new(if f.company_name.is_empty() {
                                                    "—"
                                                } else {
                                                    &f.company_name
                                                })
                                                .strong(),
                                            );
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(if f.sector.is_empty() {
                                                        "—"
                                                    } else {
                                                        &f.sector
                                                    })
                                                    .color(ACCENT)
                                                    .small(),
                                                );
                                                ui.label(
                                                    egui::RichText::new(" / ").color(AXIS_TEXT).small(),
                                                );
                                                ui.label(
                                                    egui::RichText::new(if f.industry.is_empty() {
                                                        "—"
                                                    } else {
                                                        &f.industry
                                                    })
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                                );
                                            });
                                            ui.add_space(4.0);

                                            // Valuation grid
                                            ui.label(egui::RichText::new("Valuation").small().strong());
                                            egui::Grid::new(("fund_val", ticker.as_str()))
                                                .striped(true)
                                                .num_columns(4)
                                                .show(ui, |ui| {
                                                    ui.label(
                                                        egui::RichText::new("Market Cap")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.market_cap
                                                                .map(|v| {
                                                                    fundamentals::format_large_number(v)
                                                                })
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new("Enterprise Value")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.enterprise_value
                                                                .map(|v| {
                                                                    fundamentals::format_large_number(v)
                                                                })
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.end_row();
                                                    ui.label(
                                                        egui::RichText::new("Total Debt")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.total_debt
                                                                .map(|v| {
                                                                    fundamentals::format_large_number(v)
                                                                })
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new("Cash")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.cash_and_equivalents
                                                                .map(|v| {
                                                                    fundamentals::format_large_number(v)
                                                                })
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.end_row();
                                                    ui.label(
                                                        egui::RichText::new("MCap/EV%")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    let mcev = f.mcap_ev_ratio.unwrap_or(0.0);
                                                    let mcev_col = if mcev >= 100.0 {
                                                        UP
                                                    } else if mcev < 80.0 {
                                                        DOWN
                                                    } else {
                                                        AXIS_TEXT
                                                    };
                                                    ui.label(
                                                        egui::RichText::new(format!("{:.1}%", mcev))
                                                            .color(mcev_col)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new("Stock Price")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.stock_price
                                                                .map(|v| format!("${:.2}", v))
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.end_row();
                                                    ui.label(
                                                        egui::RichText::new("Shares Out")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.shares_outstanding
                                                                .map(|v| {
                                                                    fundamentals::format_large_number(v)
                                                                })
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.end_row();
                                                });
                                            ui.add_space(4.0);

                                            // Ratios grid
                                            ui.label(egui::RichText::new("Ratios").small().strong());
                                            egui::Grid::new(("fund_ratios", ticker.as_str()))
                                                .striped(true)
                                                .num_columns(4)
                                                .show(ui, |ui| {
                                                    let pe = f.pe_ratio.unwrap_or(0.0);
                                                    let pe_col = if pe > 50.0 || pe < 0.0 {
                                                        DOWN
                                                    } else {
                                                        AXIS_TEXT
                                                    };
                                                    ui.label(
                                                        egui::RichText::new("P/E").color(AXIS_TEXT).small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.pe_ratio
                                                                .map(|v| format!("{:.1}", v))
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .color(pe_col)
                                                        .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new("Forward P/E")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.forward_pe
                                                                .map(|v| format!("{:.1}", v))
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.end_row();
                                                    ui.label(
                                                        egui::RichText::new("PEG").color(AXIS_TEXT).small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.peg_ratio
                                                                .map(|v| format!("{:.2}", v))
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new("P/B").color(AXIS_TEXT).small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.price_to_book
                                                                .map(|v| format!("{:.2}", v))
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.end_row();
                                                    ui.label(
                                                        egui::RichText::new("P/S").color(AXIS_TEXT).small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.price_to_sales
                                                                .map(|v| format!("{:.2}", v))
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new("EV/EBITDA")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.ev_to_ebitda
                                                                .map(|v| format!("{:.1}", v))
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.end_row();
                                                });
                                            ui.add_space(4.0);

                                            // Profitability grid
                                            ui.label(
                                                egui::RichText::new("Profitability & Risk")
                                                    .small()
                                                    .strong(),
                                            );
                                            egui::Grid::new(("fund_prof", ticker.as_str()))
                                                .striped(true)
                                                .num_columns(4)
                                                .show(ui, |ui| {
                                                    let margin_col =
                                                        |v: f64| if v >= 0.0 { UP } else { DOWN };
                                                    ui.label(
                                                        egui::RichText::new("Profit Margin")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    let pm = f.profit_margin.unwrap_or(0.0);
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.profit_margin
                                                                .map(|v| format!("{:.1}%", v * 100.0))
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .color(margin_col(pm))
                                                        .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new("Operating Margin")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    let om = f.operating_margin.unwrap_or(0.0);
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.operating_margin
                                                                .map(|v| format!("{:.1}%", v * 100.0))
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .color(margin_col(om))
                                                        .small(),
                                                    );
                                                    ui.end_row();
                                                    ui.label(
                                                        egui::RichText::new("ROE").color(AXIS_TEXT).small(),
                                                    );
                                                    let roe = f.roe.unwrap_or(0.0);
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.roe
                                                                .map(|v| format!("{:.1}%", v * 100.0))
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .color(margin_col(roe))
                                                        .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new("ROA").color(AXIS_TEXT).small(),
                                                    );
                                                    let roa = f.roa.unwrap_or(0.0);
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.roa
                                                                .map(|v| format!("{:.1}%", v * 100.0))
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .color(margin_col(roa))
                                                        .small(),
                                                    );
                                                    ui.end_row();
                                                    ui.label(
                                                        egui::RichText::new("Beta")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.beta
                                                                .map(|v| format!("{:.2}", v))
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new("Short Ratio")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.short_ratio
                                                                .map(|v| format!("{:.2}", v))
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.end_row();
                                                    ui.label(
                                                        egui::RichText::new("Short % Float")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.short_percent_of_float
                                                                .map(|v| format!("{:.1}%", v * 100.0))
                                                                .unwrap_or_else(|| "—".into()),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.end_row();
                                                });
                                            ui.add_space(4.0);

                                            // Earnings
                                            ui.label(egui::RichText::new("Earnings").small().strong());
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new("Next:").color(AXIS_TEXT).small(),
                                                );
                                                ui.label(
                                                    egui::RichText::new(
                                                        f.next_earnings_date.as_deref().unwrap_or("—"),
                                                    )
                                                    .small(),
                                                );
                                                ui.label(
                                                    egui::RichText::new("  Prev:").color(AXIS_TEXT).small(),
                                                );
                                                ui.label(
                                                    egui::RichText::new(
                                                        f.previous_earnings_date.as_deref().unwrap_or("—"),
                                                    )
                                                    .small(),
                                                );
                                            });

                                            // Dividends
                                            ui.label(egui::RichText::new("Dividends").small().strong());
                                            if f.is_dividend_stock {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new("Yield:")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    let dy = f.dividend_yield.unwrap_or(0.0);
                                                    let dy_col = if dy > 4.0 { UP } else { AXIS_TEXT };
                                                    ui.label(
                                                        egui::RichText::new(format!("{:.2}%", dy))
                                                            .color(dy_col)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new("  Ex-Div:")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.next_ex_dividend_date
                                                                .as_deref()
                                                                .unwrap_or("—"),
                                                        )
                                                        .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new("  Payment:")
                                                            .color(AXIS_TEXT)
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(
                                                            f.next_dividend_payment_date
                                                                .as_deref()
                                                                .unwrap_or("—"),
                                                        )
                                                        .small(),
                                                    );
                                                });
                                            } else {
                                                ui.label(
                                                    egui::RichText::new("Not a dividend stock")
                                                        .color(AXIS_TEXT)
                                                        .small(),
                                                );
                                            }
                                            ui.add_space(4.0);
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "Last updated: {}",
                                                    if f.last_updated.is_empty() {
                                                        "never"
                                                    } else {
                                                        &f.last_updated
                                                    }
                                                ))
                                                .color(AXIS_TEXT)
                                                .small(),
                                            );
                                } else {
                                    ui.label(
                                        egui::RichText::new("No fundamentals data. Click Scrape/Refresh.")
                                            .color(AXIS_TEXT),
                                    );
                                }
                                                    if tickers.len() > 1 {
                                                        ui.separator();
                                                    }
                                                });
                                                ui.add_space(8.0);
                                            } // end for ticker in visible_tickers
                                        });
                                    });
                            }
                        });
        }

        // EV Scanner
        if self.show_ev_scanner {
            let ev_active = if self.ev_active_only {
                self.cached_active_symbols.clone()
            } else {
                Vec::new()
            };
            // PERF2: read from per-frame caches — scope filter applied once already
            let scope_label = self.broker_scope_label();
            let mut ev_pending_action = SymbolAction::None;
            // UX7: Pre-fetch sparklines for visible symbols (use cached scoped — no per-row .to_uppercase())
            let visible_syms: Vec<String> = self
                .cached_scoped_fundamentals
                .iter()
                .take(200)
                .map(|f| f.symbol.clone())
                .collect();
            let mut sparklines: std::collections::HashMap<String, std::sync::Arc<Vec<f64>>> =
                std::collections::HashMap::new();
            for sym in &visible_syms {
                let closes = self.get_sparkline(sym);
                if !closes.is_empty() {
                    sparklines.insert(sym.to_uppercase(), closes);
                }
            }
            egui::Window::new("Enterprise Value Scanner")
                .open(&mut self.show_ev_scanner)
                .resizable(true)
                .default_size([900.0, 500.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Scrape All").color(egui::Color32::WHITE),
                                )
                                .fill(BTN_GREEN),
                            )
                            .clicked()
                        {
                            let db_path = cache_db_path();
                            let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                                db_path,
                                use_mt5: self.fund_source_mt5,
                                use_alpaca: self.fund_source_alpaca,
                                use_tastytrade: self.fund_source_tastytrade,
                                use_kraken: self.fund_source_kraken,
                                kraken_equity_symbols: self.kraken_equity_universe_symbols.clone(),
                                force: false,
                            });
                            self.log.push_back(LogEntry::info(
                                "Fundamentals scrape started for all MT5 symbols...",
                            ));
                        }
                        ui.label(
                            egui::RichText::new(format!(
                                "{} symbols • scope: {}",
                                self.bg.all_fundamentals.len(),
                                scope_label
                            ))
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        ui.checkbox(
                            &mut self.ev_active_only,
                            egui::RichText::new("Active Only").small(),
                        );
                    });
                    ui.separator();
                    // PERF: cached_scoped_fundamentals already applied scope filter — only need active filter
                    // O(1) HashSet lookup instead of O(n) iter().any()
                    let mut fund_sorted: Vec<&_> = self
                        .cached_scoped_fundamentals
                        .iter()
                        .filter(|f| {
                            ev_active.is_empty()
                                || self.cached_active_symbols_set.contains(f.symbol.as_str())
                        })
                        .collect();
                    match self.ev_sort.column {
                        0 => fund_sorted.sort_by(|a, b| a.symbol.cmp(&b.symbol)),
                        1 => fund_sorted.sort_by(|a, b| a.company_name.cmp(&b.company_name)),
                        2 => fund_sorted.sort_by(|a, b| {
                            a.enterprise_value
                                .unwrap_or(0.0)
                                .partial_cmp(&b.enterprise_value.unwrap_or(0.0))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        }),
                        3 => fund_sorted.sort_by(|a, b| {
                            a.market_cap
                                .unwrap_or(0.0)
                                .partial_cmp(&b.market_cap.unwrap_or(0.0))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        }),
                        4 => fund_sorted.sort_by(|a, b| {
                            a.mcap_ev_ratio
                                .unwrap_or(0.0)
                                .partial_cmp(&b.mcap_ev_ratio.unwrap_or(0.0))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        }),
                        5 => fund_sorted.sort_by(|a, b| {
                            a.pe_ratio
                                .unwrap_or(0.0)
                                .partial_cmp(&b.pe_ratio.unwrap_or(0.0))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        }),
                        6 => fund_sorted.sort_by(|a, b| {
                            a.next_earnings_date
                                .as_deref()
                                .unwrap_or("")
                                .cmp(b.next_earnings_date.as_deref().unwrap_or(""))
                        }),
                        7 => fund_sorted.sort_by(|a, b| {
                            a.dividend_yield
                                .unwrap_or(0.0)
                                .partial_cmp(&b.dividend_yield.unwrap_or(0.0))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        }),
                        8 => fund_sorted.sort_by(|a, b| a.sector.cmp(&b.sector)),
                        _ => {}
                    }
                    if !self.ev_sort.ascending {
                        fund_sorted.reverse();
                    }
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            egui::Grid::new("ev_scanner_grid")
                                .striped(true)
                                .num_columns(10)
                                .show(ui, |ui| {
                                    if SortState::header(ui, "Symbol", 0, &self.ev_sort) {
                                        self.ev_sort.toggle(0);
                                    }
                                    ui.label(egui::RichText::new("30d").color(AXIS_TEXT).small());
                                    if SortState::header(ui, "Company", 1, &self.ev_sort) {
                                        self.ev_sort.toggle(1);
                                    }
                                    if SortState::header(ui, "EV", 2, &self.ev_sort) {
                                        self.ev_sort.toggle(2);
                                    }
                                    if SortState::header(ui, "MCap", 3, &self.ev_sort) {
                                        self.ev_sort.toggle(3);
                                    }
                                    if SortState::header(ui, "MCap/EV%", 4, &self.ev_sort) {
                                        self.ev_sort.toggle(4);
                                    }
                                    if SortState::header(ui, "P/E", 5, &self.ev_sort) {
                                        self.ev_sort.toggle(5);
                                    }
                                    if SortState::header(ui, "Earnings", 6, &self.ev_sort) {
                                        self.ev_sort.toggle(6);
                                    }
                                    if SortState::header(ui, "Dividend", 7, &self.ev_sort) {
                                        self.ev_sort.toggle(7);
                                    }
                                    if SortState::header(ui, "Sector", 8, &self.ev_sort) {
                                        self.ev_sort.toggle(8);
                                    }
                                    ui.end_row();
                                    for f in &fund_sorted {
                                        let (_, ev_action) = symbol_label_with_menu(
                                            ui,
                                            &f.symbol,
                                            egui::RichText::new(&f.symbol)
                                                .small()
                                                .strong()
                                                .monospace(),
                                        );
                                        if !matches!(ev_action, SymbolAction::None) {
                                            ev_pending_action = ev_action;
                                        }
                                        // UX7: Sparkline column — f.symbol is uppercase via parse_yahoo_data.
                                        if let Some(closes) = sparklines.get(f.symbol.as_str()) {
                                            draw_inline_sparkline(ui, closes, 60.0, 14.0);
                                        } else {
                                            ui.label(
                                                egui::RichText::new("—").color(AXIS_TEXT).small(),
                                            );
                                        }
                                        ui.label(
                                            egui::RichText::new(if f.company_name.is_empty() {
                                                "—"
                                            } else {
                                                &f.company_name
                                            })
                                            .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(
                                                f.enterprise_value
                                                    .map(|v| fundamentals::format_large_number(v))
                                                    .unwrap_or_else(|| "—".into()),
                                            )
                                            .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(
                                                f.market_cap
                                                    .map(|v| fundamentals::format_large_number(v))
                                                    .unwrap_or_else(|| "—".into()),
                                            )
                                            .small(),
                                        );
                                        let mcev = f.mcap_ev_ratio.unwrap_or(0.0);
                                        let mcev_col = if mcev >= 100.0 {
                                            UP
                                        } else if mcev < 80.0 {
                                            DOWN
                                        } else {
                                            AXIS_TEXT
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("{:.1}%", mcev))
                                                .color(mcev_col)
                                                .small(),
                                        );
                                        let pe = f.pe_ratio.unwrap_or(0.0);
                                        let pe_col = if pe > 50.0 || pe < 0.0 {
                                            DOWN
                                        } else {
                                            AXIS_TEXT
                                        };
                                        ui.label(
                                            egui::RichText::new(
                                                f.pe_ratio
                                                    .map(|v| format!("{:.1}", v))
                                                    .unwrap_or_else(|| "—".into()),
                                            )
                                            .color(pe_col)
                                            .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(
                                                f.next_earnings_date.as_deref().unwrap_or("—"),
                                            )
                                            .color(AXIS_TEXT)
                                            .small(),
                                        );
                                        if f.is_dividend_stock {
                                            let dy = f.dividend_yield.unwrap_or(0.0);
                                            let dy_col = if dy > 4.0 { UP } else { AXIS_TEXT };
                                            ui.label(
                                                egui::RichText::new(format!("{:.2}%", dy))
                                                    .color(dy_col)
                                                    .small(),
                                            );
                                        } else {
                                            ui.label(
                                                egui::RichText::new("—").color(AXIS_TEXT).small(),
                                            );
                                        }
                                        ui.label(
                                            egui::RichText::new(if f.sector.is_empty() {
                                                "—"
                                            } else {
                                                &f.sector
                                            })
                                            .color(AXIS_TEXT)
                                            .small(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                });
            self.apply_symbol_action(ev_pending_action);
        }

        // Earnings Calendar
        if self.show_earnings_calendar {
            let earn_active = if self.earnings_active_only {
                self.cached_active_symbols.clone()
            } else {
                Vec::new()
            };
            let mut earn_pending_action = SymbolAction::None;
            egui::Window::new("Earnings Calendar")
                .open(&mut self.show_earnings_calendar)
                .resizable(true)
                .default_size([500.0, 400.0])
                .max_size([500.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} upcoming earnings",
                                self.bg.upcoming_earnings.len()
                            ))
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        ui.checkbox(
                            &mut self.earnings_active_only,
                            egui::RichText::new("Active Only").small(),
                        );
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            egui::Grid::new("earnings_cal_grid")
                                .striped(true)
                                .num_columns(3)
                                .show(ui, |ui| {
                                    ui.strong("Date");
                                    ui.strong("Symbol");
                                    ui.strong("Company");
                                    ui.end_row();
                                    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                                    for (sym, company, date) in &self.bg.upcoming_earnings {
                                        // PERF: fundamentals.symbol is always uppercase (parse_yahoo_data).
                                        if !earn_active.is_empty()
                                            && !self
                                                .cached_active_symbols_set
                                                .contains(sym.as_str())
                                        {
                                            continue;
                                        }
                                        let days_away =
                                            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                                                .ok()
                                                .and_then(|d| {
                                                    chrono::NaiveDate::parse_from_str(
                                                        &today, "%Y-%m-%d",
                                                    )
                                                    .ok()
                                                    .map(|t| (d - t).num_days())
                                                });
                                        let date_col = match days_away {
                                            Some(d) if d <= 3 => DOWN,
                                            Some(d) if d <= 7 => SMA200_COL,
                                            _ => AXIS_TEXT,
                                        };
                                        ui.label(egui::RichText::new(date).color(date_col).small());
                                        let (_, ec_action) = symbol_label_with_menu(
                                            ui,
                                            sym,
                                            egui::RichText::new(sym).small().strong().monospace(),
                                        );
                                        if !matches!(ec_action, SymbolAction::None) {
                                            earn_pending_action = ec_action;
                                        }
                                        ui.label(egui::RichText::new(company).small());
                                        ui.end_row();
                                    }
                                });
                        });
                });
            self.apply_symbol_action(earn_pending_action);
        }
    }
}
