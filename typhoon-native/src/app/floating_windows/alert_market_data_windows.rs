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
                            let key = format!("{:.8}|{}", price, label);
                            if self.alerts_set.insert(key) {
                                self.alerts.push((price, label));
                            }
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
                            if idx < self.alerts.len() {
                                let (p, l) = &self.alerts[idx];
                                let key = format!("{:.8}|{}", p, l);
                                self.alerts_set.remove(&key);
                            }
                            self.alerts.remove(idx);
                        }
                        if ui.button("Clear All Alerts").clicked() {
                            self.alerts.clear();
                            self.alerts_set.clear();
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
    }
}
