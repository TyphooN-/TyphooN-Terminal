use super::*;

impl TyphooNApp {
    pub(super) fn render_alert_market_data_windows(&mut self, ctx: &egui::Context) {
        // Price Alerts
        if self.show_alerts {
            egui::Window::new("Price Alerts")
                .open(&mut self.show_alerts)
                .resizable(true)
                .default_size([500.0, 350.0])
                .show(ctx, |ui| {
                    ui.heading("Alerts");
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Price:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.alert_price_input)
                                .desired_width(100.0),
                        );
                        ui.label("Label:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.alert_label_input)
                                .desired_width(100.0),
                        );
                    });
                    if ui.button("Add Alert").clicked() {
                        if let Ok(price) = self.alert_price_input.parse::<f64>() {
                            let label = if self.alert_label_input.is_empty() {
                                format_price(price)
                            } else {
                                self.alert_label_input.clone()
                            };
                            self.alerts.push((price, label));
                            self.alert_price_input.clear();
                            self.alert_label_input.clear();
                            self.log.push_back(LogEntry::info(format!(
                                "Alert set at {}",
                                format_price(price)
                            )));
                        }
                    }
                    ui.separator();
                    if self.alerts.is_empty() {
                        ui.label(egui::RichText::new("No alerts set.").color(AXIS_TEXT));
                    } else {
                        let mut remove_idx: Option<usize> = None;
                        for (i, (price, label)) in self.alerts.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format_price(*price))
                                        .strong()
                                        .monospace(),
                                );
                                ui.label(label);
                                if ui.small_button("X").clicked() {
                                    remove_idx = Some(i);
                                }
                            });
                        }
                        if let Some(idx) = remove_idx {
                            self.alerts.remove(idx);
                        }
                        if ui.button("Clear All Alerts").clicked() {
                            self.alerts.clear();
                        }
                    }

                    // Check alerts against current price
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        if let Some(last) = chart.bars.last() {
                            for (price, label) in &self.alerts {
                                let dist = (last.close - price).abs();
                                let pct = dist / last.close * 100.0;
                                if pct < 0.1 {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "ALERT TRIGGERED: {} at {}",
                                            label,
                                            format_price(*price)
                                        ))
                                        .color(egui::Color32::from_rgb(255, 80, 80))
                                        .strong(),
                                    );
                                }
                            }
                        }
                    }
                    // ── DARWIN Risk Alerts ──────────────────────
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("DARWIN Risk Alerts").strong());
                    ui.separator();
                    {
                        let alerts = &self.bg.darwin_alerts;
                        if alerts.is_empty() {
                            ui.label(egui::RichText::new("No risk alerts — all clear.").color(UP));
                        } else {
                            for alert in alerts {
                                let color = match alert.severity.as_str() {
                                    "CRITICAL" => DOWN,
                                    "WARNING" => egui::Color32::from_rgb(255, 200, 50),
                                    _ => AXIS_TEXT,
                                };
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("\u{2588}").color(color));
                                    ui.label(
                                        egui::RichText::new(&alert.severity)
                                            .color(color)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&alert.alert_type).small().strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&alert.message)
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                });
                            }
                        }
                    }
                });
        }

        // Fear & Greed Index window
        if self.show_fear_greed {
            egui::Window::new("Fear & Greed Index")
                .open(&mut self.show_fear_greed)
                .resizable(true)
                .default_size([340.0, 220.0])
                .show(ctx, |ui| {
                    ui.heading("Crypto Fear & Greed Index");
                    ui.separator();
                    if ui.button("Refresh").clicked() {
                        let _ = self.broker_tx.send(BrokerCmd::FetchFearGreed);
                    }
                    ui.add_space(8.0);

                    let val = self.fear_greed_value;
                    // Color based on value zone
                    let gauge_color = if val <= 25 {
                        egui::Color32::from_rgb(255, 50, 50) // Extreme Fear — red
                    } else if val <= 45 {
                        egui::Color32::from_rgb(255, 165, 0) // Fear — orange
                    } else if val <= 55 {
                        egui::Color32::from_rgb(255, 255, 80) // Neutral — yellow
                    } else if val <= 75 {
                        egui::Color32::from_rgb(144, 238, 100) // Greed — light green
                    } else {
                        egui::Color32::from_rgb(0, 200, 0) // Extreme Greed — green
                    };

                    // Large value display
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{}", val))
                                .color(gauge_color)
                                .size(48.0)
                                .strong(),
                        );
                        ui.vertical(|ui| {
                            ui.add_space(12.0);
                            ui.label(
                                egui::RichText::new(&self.fear_greed_label)
                                    .color(gauge_color)
                                    .size(18.0),
                            );
                            ui.label(egui::RichText::new("/ 100").color(AXIS_TEXT).size(14.0));
                        });
                    });

                    ui.add_space(8.0);

                    // Gauge bar
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 24.0),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter_at(rect);
                    // Background
                    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(40, 40, 40));
                    // Gradient zones
                    let w = rect.width();
                    let zone_colors = [
                        (0.0, 0.25, egui::Color32::from_rgb(255, 50, 50)),
                        (0.25, 0.45, egui::Color32::from_rgb(255, 165, 0)),
                        (0.45, 0.55, egui::Color32::from_rgb(255, 255, 80)),
                        (0.55, 0.75, egui::Color32::from_rgb(144, 238, 100)),
                        (0.75, 1.0, egui::Color32::from_rgb(0, 200, 0)),
                    ];
                    for (start, end, color) in &zone_colors {
                        let zone_rect = egui::Rect::from_min_max(
                            egui::pos2(rect.min.x + w * *start as f32, rect.min.y),
                            egui::pos2(rect.min.x + w * *end as f32, rect.max.y),
                        );
                        painter.rect_filled(zone_rect, 0.0, *color);
                    }
                    // Indicator needle
                    let needle_x = rect.min.x + w * (val as f32 / 100.0);
                    painter.line_segment(
                        [
                            egui::pos2(needle_x, rect.min.y - 2.0),
                            egui::pos2(needle_x, rect.max.y + 2.0),
                        ],
                        egui::Stroke::new(3.0, egui::Color32::WHITE),
                    );

                    ui.add_space(4.0);
                    // Zone labels
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Extreme Fear")
                                .color(egui::Color32::from_rgb(255, 50, 50))
                                .small(),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Fear")
                                .color(egui::Color32::from_rgb(255, 165, 0))
                                .small(),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Neutral")
                                .color(egui::Color32::from_rgb(255, 255, 80))
                                .small(),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Greed")
                                .color(egui::Color32::from_rgb(144, 238, 100))
                                .small(),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Extreme Greed")
                                .color(egui::Color32::from_rgb(0, 200, 0))
                                .small(),
                        );
                    });
                });
        }

        // World Indices Dashboard
        if self.show_world_indices {
            egui::Window::new("World Indices")
                .open(&mut self.show_world_indices)
                .resizable(true)
                .default_size([620.0, 480.0])
                .max_size([620.0, 560.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("World Stock Indices & ETFs").strong());
                    if ui.small_button("Refresh").clicked() {
                        let symbols = vec![
                            "DIA", "SPY", "QQQ", "IWM", "EFA", "EEM", "VGK", "EWJ", "FXI", "EWZ",
                            "GLD", "SLV", "USO", "TLT", "UUP", "BTCUSD",
                        ]
                        .into_iter()
                        .map(String::from)
                        .collect();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::GetWatchlistQuotes { symbols });
                    }
                    ui.separator();
                    if self.world_indices_data.is_empty() {
                        ui.label(
                            egui::RichText::new("Loading... (requires broker connection)")
                                .color(AXIS_TEXT),
                        );
                    } else {
                        let descs: std::collections::HashMap<&str, &str> = [
                            ("DIA", "DJIA"),
                            ("SPY", "S&P 500"),
                            ("QQQ", "NASDAQ-100"),
                            ("IWM", "Russell 2000"),
                            ("EFA", "EAFE Intl"),
                            ("EEM", "Emerging Mkts"),
                            ("VGK", "Europe"),
                            ("EWJ", "Japan"),
                            ("FXI", "China"),
                            ("EWZ", "Brazil"),
                            ("GLD", "Gold"),
                            ("SLV", "Silver"),
                            ("USO", "Oil"),
                            ("TLT", "20Y Bonds"),
                            ("UUP", "US Dollar"),
                            ("BTCUSD", "Bitcoin"),
                        ]
                        .iter()
                        .cloned()
                        .collect();
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                egui::Grid::new("indices_grid")
                                    .striped(true)
                                    .num_columns(5)
                                    .min_col_width(80.0)
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("Symbol")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Name")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Last")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Change")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Change%")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.end_row();
                                        for row in &self.world_indices_data {
                                            let desc = descs
                                                .get(row.symbol.to_uppercase().as_str())
                                                .unwrap_or(&"");
                                            let color = if row.change_pct > 0.0 {
                                                UP
                                            } else if row.change_pct < 0.0 {
                                                DOWN
                                            } else {
                                                AXIS_TEXT
                                            };
                                            ui.label(
                                                egui::RichText::new(&row.symbol)
                                                    .small()
                                                    .strong()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(*desc).small().color(AXIS_TEXT),
                                            );
                                            ui.label(
                                                egui::RichText::new(format_price(row.last))
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:+.2}", row.change))
                                                    .color(color)
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:+.2}%",
                                                    row.change_pct
                                                ))
                                                .color(color)
                                                .small()
                                                .strong()
                                                .monospace(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
        }

        // Crypto Top 50 (CoinGecko)
        if self.show_crypto_top50 {
            egui::Window::new("Crypto Top 50")
                .open(&mut self.show_crypto_top50)
                .resizable(true)
                .default_size([700.0, 550.0])
                .max_size([700.0, 560.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("Top 50 Cryptocurrencies by Market Cap").strong());
                    if ui.small_button("Refresh").clicked() {
                        let _ = self.broker_tx.send(BrokerCmd::FetchCryptoTop50);
                    }
                    ui.separator();
                    if self.crypto_top50.is_empty() {
                        ui.label(egui::RichText::new("Loading from CoinGecko...").color(AXIS_TEXT));
                    } else {
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                egui::Grid::new("crypto50_grid")
                                    .striped(true)
                                    .num_columns(5)
                                    .min_col_width(80.0)
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("#")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Name")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Price")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("24h%")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Market Cap")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.end_row();
                                        for (i, (name, price, change, mcap)) in
                                            self.crypto_top50.iter().enumerate()
                                        {
                                            let color = if *change > 0.0 {
                                                UP
                                            } else if *change < 0.0 {
                                                DOWN
                                            } else {
                                                AXIS_TEXT
                                            };
                                            ui.label(
                                                egui::RichText::new(format!("{}", i + 1))
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(egui::RichText::new(name).small());
                                            let price_str = if *price >= 1.0 {
                                                format!("${:.2}", price)
                                            } else {
                                                format!("${:.6}", price)
                                            };
                                            ui.label(
                                                egui::RichText::new(price_str).small().monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:+.2}%", change))
                                                    .color(color)
                                                    .small()
                                                    .strong()
                                                    .monospace(),
                                            );
                                            let mcap_str = if *mcap >= 1e12 {
                                                format!("${:.1}T", mcap / 1e12)
                                            } else if *mcap >= 1e9 {
                                                format!("${:.1}B", mcap / 1e9)
                                            } else if *mcap >= 1e6 {
                                                format!("${:.1}M", mcap / 1e6)
                                            } else {
                                                format!("${:.0}", mcap)
                                            };
                                            ui.label(
                                                egui::RichText::new(mcap_str).small().monospace(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
        }

        // Forex Major Pairs Dashboard
        if self.show_forex_matrix {
            egui::Window::new("Forex Pairs")
                .open(&mut self.show_forex_matrix)
                .resizable(true)
                .default_size([550.0, 380.0])
                .max_size([550.0, 560.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("Major Forex Pairs").strong());
                    if ui.small_button("Refresh").clicked() {
                        let symbols = vec![
                            "EURUSD", "GBPUSD", "USDJPY", "USDCHF", "AUDUSD", "NZDUSD", "USDCAD",
                            "EURGBP", "EURJPY", "GBPJPY",
                        ]
                        .into_iter()
                        .map(String::from)
                        .collect();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::GetWatchlistQuotes { symbols });
                    }
                    ui.separator();
                    if self.forex_pairs_data.is_empty() {
                        ui.label(
                            egui::RichText::new("Loading... (requires broker connection)")
                                .color(AXIS_TEXT),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                egui::Grid::new("forex_grid")
                                    .striped(true)
                                    .num_columns(4)
                                    .min_col_width(90.0)
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("Pair")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Last")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Change")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Change%")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.end_row();
                                        for row in &self.forex_pairs_data {
                                            let color = if row.change_pct > 0.0 {
                                                UP
                                            } else if row.change_pct < 0.0 {
                                                DOWN
                                            } else {
                                                AXIS_TEXT
                                            };
                                            // Forex uses 5 decimal places for most, 3 for JPY pairs
                                            let is_jpy = row.symbol.to_uppercase().contains("JPY");
                                            let price_str = if is_jpy {
                                                format!("{:.3}", row.last)
                                            } else {
                                                format!("{:.5}", row.last)
                                            };
                                            ui.label(
                                                egui::RichText::new(&row.symbol)
                                                    .small()
                                                    .strong()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(price_str).small().monospace(),
                                            );
                                            let chg_str = if is_jpy {
                                                format!("{:+.3}", row.change)
                                            } else {
                                                format!("{:+.5}", row.change)
                                            };
                                            ui.label(
                                                egui::RichText::new(chg_str)
                                                    .color(color)
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:+.2}%",
                                                    row.change_pct
                                                ))
                                                .color(color)
                                                .small()
                                                .strong()
                                                .monospace(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
        }

        // DARWIN FTP Browser
        if self.show_darwin_browser {
            egui::Window::new("DarwinIA Browser")
                        .open(&mut self.show_darwin_browser)
                        .resizable(true).default_size([950.0, 600.0])
        .max_size([950.0, 640.0])
                        .show(ctx, |ui| {
                            // Top bar: scan button + stats
                            ui.horizontal(|ui| {
                                let has_gpu = self.gpu_darwin.is_some();
                                let label = if has_gpu { "DarwinIA Scan (GPU)" } else { "DarwinIA Scan (CPU)" };
                                if ui.add_enabled(!self.darwin_ftp_dir.is_empty(), egui::Button::new(label)).clicked() {
                                    if has_gpu {
                                        let _ = self.broker_tx.send(BrokerCmd::DarwinGpuScan { ftp_dir: self.darwin_ftp_dir.clone(), min_days: 90 });
                                        self.log.push_back(LogEntry::info("DarwinIA scan started (GPU)..."));
                                    } else {
                                        let _ = self.broker_tx.send(BrokerCmd::DarwinFtpScan { ftp_dir: self.darwin_ftp_dir.clone(), min_days: 90 });
                                        self.log.push_back(LogEntry::info("DarwinIA scan started (CPU)..."));
                                    }
                                }
                                ui.label(format!("{} DARWINs loaded", self.ftp_scan_results.len()));
                                ui.separator();
                                // Ticker lookup
                                ui.label("Lookup:");
                                ui.add(egui::TextEdit::singleline(&mut self.ftp_detail_ticker).desired_width(60.0).hint_text("HAKR"));
                                if ui.button("View").clicked() && !self.ftp_detail_ticker.is_empty() && !self.darwin_ftp_dir.is_empty() {
                                    let ticker = self.ftp_detail_ticker.trim().to_uppercase();
                                    self.ftp_detail_ticker = ticker.clone();
                                    let ftp = std::path::Path::new(&self.darwin_ftp_dir);
                                    self.ftp_detail_avail = Some(darwin_ftp::check_availability(ftp, &ticker));
                                    if let Ok(returns) = darwin_ftp::read_return_file(ftp, &ticker) {
                                        self.ftp_detail_summary = Some(darwin_ftp::compute_return_summary(&ticker, &returns));
                                        self.ftp_detail_returns = returns;
                                    } else {
                                        self.ftp_detail_summary = None;
                                        self.ftp_detail_returns.clear();
                                    }
                                }
                            });
                            ui.separator();

                            // Two-panel layout: left = table, right = detail
                            let avail_width = ui.available_width();
                            ui.horizontal(|ui| {
                                // Left panel: scan results table
                                ui.vertical(|ui| {
                                    ui.set_width(avail_width * 0.55);
                                    ui.heading("Universe");
                                    if self.ftp_scan_results.is_empty() {
                                        ui.label(egui::RichText::new("Click 'Scan Universe' to load DARWINs from FTP.").color(AXIS_TEXT));
                                        if self.darwin_ftp_dir.is_empty() {
                                            ui.label(egui::RichText::new("Set FTP Dir in Settings first.").color(DOWN));
                                        }
                                    } else {
                                        let mut darwin_sorted: Vec<&_> = self.ftp_scan_results.iter().collect();
                                        match self.darwin_browser_sort.column {
                                            0 => darwin_sorted.sort_by(|a, b| a.ticker.cmp(&b.ticker)),
                                            1 => darwin_sorted.sort_by(|a, b| a.trading_days.cmp(&b.trading_days)),
                                            2 => darwin_sorted.sort_by(|a, b| a.total_return_pct.partial_cmp(&b.total_return_pct).unwrap_or(std::cmp::Ordering::Equal)),
                                            3 => darwin_sorted.sort_by(|a, b| a.max_drawdown_pct.partial_cmp(&b.max_drawdown_pct).unwrap_or(std::cmp::Ordering::Equal)),
                                            4 => darwin_sorted.sort_by(|a, b| a.sharpe.partial_cmp(&b.sharpe).unwrap_or(std::cmp::Ordering::Equal)),
                                            5 => darwin_sorted.sort_by(|a, b| a.sortino.partial_cmp(&b.sortino).unwrap_or(std::cmp::Ordering::Equal)),
                                            6 => darwin_sorted.sort_by(|a, b| a.last_quote.partial_cmp(&b.last_quote).unwrap_or(std::cmp::Ordering::Equal)),
                                            _ => {}
                                        }
                                        if !self.darwin_browser_sort.ascending { darwin_sorted.reverse(); }
                                        egui::ScrollArea::vertical().auto_shrink(false).max_height(500.0).show(ui, |ui| {
                                            egui::Grid::new("ftp_universe").striped(true).num_columns(7).show(ui, |ui| {
                                                if SortState::header(ui, "DARWIN", 0, &self.darwin_browser_sort) { self.darwin_browser_sort.toggle(0); }
                                                if SortState::header(ui, "Days", 1, &self.darwin_browser_sort) { self.darwin_browser_sort.toggle(1); }
                                                if SortState::header(ui, "Return%", 2, &self.darwin_browser_sort) { self.darwin_browser_sort.toggle(2); }
                                                if SortState::header(ui, "MaxDD%", 3, &self.darwin_browser_sort) { self.darwin_browser_sort.toggle(3); }
                                                if SortState::header(ui, "Sharpe", 4, &self.darwin_browser_sort) { self.darwin_browser_sort.toggle(4); }
                                                if SortState::header(ui, "Sortino", 5, &self.darwin_browser_sort) { self.darwin_browser_sort.toggle(5); }
                                                if SortState::header(ui, "Price", 6, &self.darwin_browser_sort) { self.darwin_browser_sort.toggle(6); }
                                                ui.end_row();
                                                for s in darwin_sorted.iter().take(500) {
                                                    let ret_c = if s.total_return_pct >= 0.0 { UP } else { DOWN };
                                                    // Clickable ticker
                                                    if ui.add(egui::Label::new(egui::RichText::new(&s.ticker).strong().color(ACCENT)).sense(egui::Sense::click())).clicked() {
                                                        self.ftp_detail_ticker = s.ticker.clone();
                                                        let ftp = std::path::Path::new(&self.darwin_ftp_dir);
                                                        self.ftp_detail_avail = Some(darwin_ftp::check_availability(ftp, &s.ticker));
                                                        if let Ok(returns) = darwin_ftp::read_return_file(ftp, &s.ticker) {
                                                            self.ftp_detail_summary = Some(darwin_ftp::compute_return_summary(&s.ticker, &returns));
                                                            self.ftp_detail_returns = returns;
                                                        }
                                                    }
                                                    ui.label(format!("{}", s.trading_days));
                                                    ui.label(egui::RichText::new(format!("{:.1}%", s.total_return_pct)).color(ret_c));
                                                    ui.label(egui::RichText::new(format!("{:.1}%", s.max_drawdown_pct)).color(DOWN));
                                                    let sharpe_c = if s.sharpe >= 1.0 { UP } else if s.sharpe >= 0.0 { AXIS_TEXT } else { DOWN };
                                                    ui.label(egui::RichText::new(format!("{:.2}", s.sharpe)).color(sharpe_c));
                                                    ui.label(format!("{:.2}", s.sortino));
                                                    ui.label(format!("{:.1}", s.last_quote));
                                                    ui.end_row();
                                                }
                                            });
                                        });
                                    }
                                });

                                ui.separator();

                                // Right panel: detail view
                                ui.vertical(|ui| {
                                    ui.set_width(avail_width * 0.42);
                                    if let Some(ref summary) = self.ftp_detail_summary {
                                        ui.heading(format!("DARWIN {}", summary.ticker));
                                        ui.separator();
                                        egui::Grid::new("ftp_detail").striped(true).num_columns(2).show(ui, |ui| {
                                            ui.label("Trading Days:"); ui.label(format!("{}", summary.trading_days)); ui.end_row();
                                            let ret_c = if summary.total_return_pct >= 0.0 { UP } else { DOWN };
                                            ui.label("Total Return:"); ui.label(egui::RichText::new(format!("{:.2}%", summary.total_return_pct)).color(ret_c)); ui.end_row();
                                            ui.label("Max Drawdown:"); ui.label(egui::RichText::new(format!("{:.2}%", summary.max_drawdown_pct)).color(DOWN)); ui.end_row();
                                            ui.label("Sharpe Ratio:"); ui.label(format!("{:.3}", summary.sharpe)); ui.end_row();
                                            ui.label("Sortino Ratio:"); ui.label(format!("{:.3}", summary.sortino)); ui.end_row();
                                            ui.label("Daily Vol:"); ui.label(format!("{:.4}", summary.daily_vol)); ui.end_row();
                                            ui.label("Best Day:"); ui.label(egui::RichText::new(format!("{:.2}%", summary.best_day_pct)).color(UP)); ui.end_row();
                                            ui.label("Worst Day:"); ui.label(egui::RichText::new(format!("{:.2}%", summary.worst_day_pct)).color(DOWN)); ui.end_row();
                                            ui.label("DARWIN Price:"); ui.label(format!("{:.2}", summary.last_quote)); ui.end_row();
                                            ui.label("Experience:"); ui.label(format!("{:.1}", summary.experience_score)); ui.end_row();
                                            ui.label("Risk Stability:"); ui.label(format!("{:.1}", summary.risk_stability_score)); ui.end_row();
                                            ui.label("Performance:"); ui.label(format!("{:.1}", summary.performance_score)); ui.end_row();
                                        });

                                        // Equity curve plot
                                        if self.ftp_detail_returns.len() > 5 {
                                            ui.add_space(10.0);
                                            ui.label(egui::RichText::new("Equity Curve").strong());
                                            let points: PlotPoints = PlotPoints::new(
                                                self.ftp_detail_returns.iter().enumerate()
                                                    .filter_map(|(i, r)| r.cumulative_returns.last().map(|v| [i as f64, *v * 100.0]))
                                                    .collect()
                                            );
                                            let line = Line::new("Equity", points).color(ACCENT);
                                            Plot::new("ftp_equity_plot")
                                                .height(180.0)
                                                .allow_drag(false)
                                                .allow_zoom(false)
                                                .show(ui, |plot_ui| { plot_ui.line(line); });
                                        }

                                        // Data availability
                                        if let Some(ref avail) = self.ftp_detail_avail {
                                            ui.add_space(10.0);
                                            ui.label(egui::RichText::new("Data Available").strong());
                                            ui.horizontal_wrapped(|ui| {
                                                let show = |ui: &mut egui::Ui, name: &str, has: bool| {
                                                    let c = if has { UP } else { egui::Color32::from_rgb(60, 60, 60) };
                                                    ui.label(egui::RichText::new(name).color(c).small());
                                                };
                                                show(ui, "RETURN", avail.has_return);
                                                show(ui, "TRADES", avail.has_trades);
                                                show(ui, "POSITIONS", avail.has_positions);
                                                show(ui, "EXPERIENCE", avail.has_experience);
                                                show(ui, "RISK", avail.has_risk_stability);
                                                show(ui, "PERF", avail.has_performance);
                                                show(ui, "SCALE", avail.has_scalability);
                                                show(ui, "CORR", avail.has_market_correlation);
                                                show(ui, "BADGES", avail.has_badges);
                                                show(ui, "QUOTES", avail.has_quotes);
                                                show(ui, "VAR10", avail.has_former_var10);
                                            });
                                            if !avail.quote_months.is_empty() {
                                                ui.label(egui::RichText::new(format!("Quotes: {} months ({} → {})",
                                                    avail.quote_months.len(),
                                                    avail.quote_months.first().unwrap_or(&String::new()),
                                                    avail.quote_months.last().unwrap_or(&String::new())
                                                )).color(AXIS_TEXT).small());
                                            }
                                            ui.label(egui::RichText::new(format!("D-Score: {} days", avail.dscore_days)).color(AXIS_TEXT).small());
                                        }

                                        // Correlation with our DARWINs
                                        if !self.bg.accounts.is_empty() && !self.darwin_ftp_dir.is_empty() {
                                            ui.add_space(10.0);
                                            ui.label(egui::RichText::new("Correlation with Portfolio").strong());
                                            let ftp = std::path::Path::new(&self.darwin_ftp_dir);
                                            for acct in &self.bg.accounts {
                                                match darwin_ftp::compute_correlation(ftp, &summary.ticker, &acct.darwin_ticker) {
                                                    Ok(corr) => {
                                                        let c = if corr.abs() > 0.7 { DOWN } else if corr.abs() > 0.4 { egui::Color32::from_rgb(255, 200, 50) } else { UP };
                                                        ui.label(egui::RichText::new(format!("vs {}: {:.4}", acct.darwin_ticker, corr)).color(c).small());
                                                    }
                                                    Err(_) => {
                                                        ui.label(egui::RichText::new(format!("vs {}: N/A", acct.darwin_ticker)).color(AXIS_TEXT).small());
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        ui.heading("DARWIN Detail");
                                        ui.label(egui::RichText::new("Enter a ticker and click View, or click a ticker in the table.").color(AXIS_TEXT));
                                    }
                                });
                            });
                        });
        }
    }
}
