use super::*;
mod bookmap;
mod broker_darwin_windows;
use bookmap::*;
mod news_filter;
use news_filter::*;
mod bardata;
mod macro_windows;
mod market_analytics_windows;
mod matrix_chat;
mod news;
mod reddit;
mod research_adr107;
mod research_ingest;
mod research_round02;
mod research_round03;
mod research_round04;
mod research_round05;
mod research_round06;
mod research_round07;
mod research_round08;
mod research_round09;
mod research_round10;
mod research_round11;
mod research_round12;
mod research_round13_to15;
mod research_round16;
mod research_round17;
mod research_round18_to20;
mod research_round21_to22;
mod research_round23;
mod research_round24;
mod research_round25;
mod research_round26;
mod research_round27;
mod research_round28;
mod research_round29;
mod research_round30;
mod research_round31;
mod research_round32;
mod research_round33;
mod research_round34;
mod research_round35;
mod research_round36;
mod research_round37;
mod research_round38;
mod research_round39;
mod research_round40;
mod research_round41;
mod research_round42;
mod research_round43;
mod research_round44;
mod research_round46;
mod research_round47;
mod research_round48;
mod research_round51;
mod research_round52;
mod research_round55;
mod research_round60;
mod research_round61;
mod research_round62;
mod research_round63;
mod research_round64;
mod research_round66;
mod research_round67;
mod research_round68;
mod research_round71;
mod research_round72;
mod research_round76;
mod research_round77;
mod research_round78;
mod risk_journal_windows;
mod scope;
mod scrape_darwinia_windows;
mod screenshots;
mod sec_calendar_windows;
mod storage_sync_windows;
mod symbol_explorer;
mod symbol_screener;
mod trading_tools_windows;
mod workspace_reference_windows;

fn sortable_header(
    ui: &mut egui::Ui,
    label: &str,
    col: usize,
    sort_col: &mut usize,
    sort_asc: &mut bool,
) {
    let arrow = if *sort_col == col {
        if *sort_asc { " ↑" } else { " ↓" }
    } else {
        ""
    };
    if ui
        .add(egui::Button::new(
            egui::RichText::new(format!("{label}{arrow}"))
                .small()
                .strong(),
        ))
        .on_hover_text("Sort by this column")
        .clicked()
    {
        if *sort_col == col {
            *sort_asc = !*sort_asc;
        } else {
            *sort_col = col;
            *sort_asc = true;
        }
    }
}

impl TyphooNApp {
    pub(super) fn draw_floating_windows(&mut self, ctx: &egui::Context) {
        // Settings
        // Save credentials to keyring + SQLite fallback when Settings window closes
        if self.was_settings_open && !self.show_settings {
            let creds = [
                (keyring::keys::ALPACA_API_KEY, self.broker_api_key.as_str()),
                (keyring::keys::ALPACA_SECRET, self.broker_secret.as_str()),
                (keyring::keys::FINNHUB_KEY, self.finnhub_key.as_str()),
                (keyring::keys::FRED_KEY, self.fred_key.as_str()),
                (keyring::keys::TT_USERNAME, self.tt_username.as_str()),
                (keyring::keys::TT_PASSWORD, self.tt_password.as_str()),
                (
                    keyring::keys::LAN_SYNC_PASS,
                    self.lan_sync_passphrase.as_str(),
                ),
                (
                    keyring::keys::DISCORD_WEBHOOK,
                    self.discord_webhook.as_str(),
                ),
                (keyring::keys::PUSHOVER_TOKEN, self.pushover_token.as_str()),
                (keyring::keys::PUSHOVER_USER, self.pushover_user.as_str()),
                (keyring::keys::NTFY_TOPIC, self.ntfy_topic.as_str()),
                (keyring::keys::ANTHROPIC_KEY, self.anthropic_key.as_str()),
                (keyring::keys::OPENAI_KEY, self.openai_key.as_str()),
                (keyring::keys::KRAKEN_API_KEY, self.kraken_api_key.as_str()),
                (
                    keyring::keys::KRAKEN_API_SECRET,
                    self.kraken_api_secret.as_str(),
                ),
                (
                    keyring::keys::KRAKEN_WS_API_KEY,
                    self.kraken_ws_api_key.as_str(),
                ),
                (
                    keyring::keys::KRAKEN_WS_API_SECRET,
                    self.kraken_ws_api_secret.as_str(),
                ),
            ];
            let mut kr_ok = true;
            let mut saved_credentials: Vec<&'static str> = Vec::new();
            for (key, val) in &creds {
                if let Err(e) = keyring::store(key, val) {
                    kr_ok = false;
                    self.log.push_back(LogEntry::warn(format!(
                        "Keyring store '{}' failed: {}",
                        key, e
                    )));
                } else {
                    saved_credentials.push(*key);
                }
                // Always write SQLite fallback
                if let Some(ref cache) = self.cache {
                    let _ = cache.put_kv(&format!("cred:{}", key), val);
                }
            }
            let dest = if kr_ok {
                "system keyring + SQLite"
            } else {
                "SQLite fallback (keyring unavailable)"
            };
            if !saved_credentials.is_empty() {
                self.log.push_back(LogEntry::info(format!(
                    "Credentials saved to {}: {}",
                    dest,
                    saved_credentials.join(", ")
                )));
            }
            // Also save session to persist non-credential settings (tt_sandbox, broker_paper, etc.)
            self.save_session();
        }
        self.was_settings_open = self.show_settings;

        let _settings_save_after = self.render_settings_window(ctx);
        // Broker, Kraken, and Darwinex windows
        self.render_broker_darwin_windows(ctx);

        // AI Chat (Anthropic Claude / OpenAI GPT / …)
        self.render_ai_chat_window(ctx);

        // ── Claude Code CLI chat ──
        self.render_claude_code_window(ctx);

        // ── Gemini CLI chat ──
        self.render_gemini_cli_window(ctx);

        // ── Codex CLI chat ──
        self.render_codex_cli_window(ctx);

        // ── Hermes Agent CLI chat ──
        self.render_hermes_cli_window(ctx);

        // ── Grok Build CLI chat ──
        self.render_grok_cli_window(ctx);

        // ── AI Sessions history browser ──
        self.render_ai_sessions_window(ctx);

        // ── Screenshots Gallery (palette: SCREENSHOTS / GALLERY) ──
        self.render_screenshots_gallery_window(ctx);

        // ── AI Response Cache stats window ──
        self.render_ai_cache_window(ctx);

        // Matrix Chat (public room viewer)
        self.render_matrix_chat_window(ctx);

        // BARDATA Progress Window
        self.render_bardata_progress_window(ctx);

        // Reddit WallStreetBets
        self.render_reddit_window(ctx);

        // Risk Calculator — wired to engine risk.rs
        // ── SCOPE popup window with source checkboxes ──
        self.render_scope_window(ctx);

        self.render_risk_calc_window(ctx);
        self.render_compound_calc_window(ctx);

        self.render_backtest_window(ctx);

        // Screener — uses cached symbol data
        self.render_symbol_screener_window(ctx);

        // Symbols Explorer — all-encompassing symbol browser with broker hierarchy
        self.render_symbol_explorer_window(ctx);

        self.render_optimizer_window(ctx);

        // News
        self.render_news_window(ctx);

        // ── Godel parity research windows (ADR-107) ───────────────────────
        self.render_research_adr107_windows(ctx);

        // ── Research Godel Parity Round 2 windows ─────────────────────
        self.render_research_round02_windows(ctx);

        // ── Research Godel Parity Round 3 windows ─────────────────────
        self.render_research_round03_windows(ctx);

        // ── Research Round 4 windows ──────────────────────────────────
        self.render_research_round04_windows(ctx);

        // ── Research Round 5 windows ──────────────────────────────────
        self.render_research_round05_windows(ctx);

        // ── Research Round 6 windows ──────────────────────────────────
        self.render_research_round06_windows(ctx);

        // ── Research Godel Parity Round 7 ──
        self.render_research_round07_windows(ctx);

        // ── Research Round 8 windows ──
        self.render_research_round08_windows(ctx);

        // ── Research Round 9 windows ──
        self.render_research_round09_windows(ctx);

        // ── Research Godel Parity Round 10 ──
        self.render_research_round10_windows(ctx);

        // ── Research Godel Parity Round 11 windows ─────────────────────────────
        self.render_research_round11_windows(ctx);

        // Research Round 12 windows
        self.render_research_round12_windows(ctx);

        // Research Rounds 13-15 windows
        self.render_research_round13_to15_windows(ctx);

        // ── Research Round 16 ────────────────────────────────────────────────
        self.render_research_round16_windows(ctx);

        // ── Research Round 17 ──
        self.render_research_round17_windows(ctx);

        // Research Rounds 18-20 windows
        self.render_research_round18_to20_windows(ctx);

        // Research Rounds 21-22 windows
        self.render_research_round21_to22_windows(ctx);

        // ── Research Round 23 windows ──
        self.render_research_round23_windows(ctx);

        // ── Research Round 24 windows ──
        self.render_research_round24_windows(ctx);

        // ── Research Round 25 windows ──
        self.render_research_round25_windows(ctx);

        // ── Research Round 26 windows ──
        self.render_research_round26_windows(ctx);

        // ── Research Round 27 windows ──
        self.render_research_round27_windows(ctx);

        // ── Research Round 28 windows ──
        self.render_research_round28_windows(ctx);

        // ── Research Round 29 windows ──
        self.render_research_round29_windows(ctx);

        // ── Research Round 30 windows ──
        self.render_research_round30_windows(ctx);

        // ── Research Round 31 windows ──
        self.render_research_round31_windows(ctx);

        // ── Research Round 32 windows ──
        self.render_research_round32_windows(ctx);

        // ── Research Round 33 windows ──
        self.render_research_round33_windows(ctx);

        // ── Research Round 34 windows ──
        self.render_research_round34_windows(ctx);

        // ── Research Round 35 windows ──
        self.render_research_round35_windows(ctx);

        // ── Research Round 36 windows ──
        self.render_research_round36_windows(ctx);

        // ── Research Round 37 windows ──
        self.render_research_round37_windows(ctx);

        // ── Research Round 38 windows ──
        self.render_research_round38_windows(ctx);

        // ── Research Round 39 windows ──
        self.render_research_round39_windows(ctx);

        // ── Research Round 40 windows ──
        self.render_research_round40_windows(ctx);

        // ── Research Round 41 windows ──
        self.render_research_round41_windows(ctx);

        // ── Research Round 42 windows ──
        self.render_research_round42_windows(ctx);

        // ── Research Round 43 windows ──
        self.render_research_round43_windows(ctx);

        // ── Research Round 44 windows ──
        self.render_research_round44_windows(ctx);

        // ── Research Round 46 windows ──
        self.render_research_round46_windows(ctx);

        // ── Research Round 47 windows ──
        self.render_research_round47_windows(ctx);

        // ── Research Round 48 windows ──
        self.render_research_round48_windows(ctx);

        // ── Research Round 51 windows ──
        self.render_research_round51_windows(ctx);

        // ── Research Round 52 windows ──
        self.render_research_round52_windows(ctx);

        // ── Research Round 55: SMMA / ALLIGATOR / CRSI / SEB / IMI ──
        self.render_research_round55_windows(ctx);

        // ── Research Round 60: WMA / RAINBOW / MESA_SINE / FRAMA / IBS windows ──
        self.render_research_round60_windows(ctx);

        // ── Research Round 61: LAGUERRE_RSI / ZIGZAG / PGO / HT_TRENDLINE / MIDPOINT windows ──
        self.render_research_round61_windows(ctx);

        // ── Research Round 62 windows ──
        self.render_research_round62_windows(ctx);

        // ── Research Round 63 egui windows ──
        self.render_research_round63_windows(ctx);

        // ── Research Round 64 egui windows ──
        self.render_research_round64_windows(ctx);

        // ── Research Round 66 windows: AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE ──
        self.render_research_round66_windows(ctx);

        // ── Research Round 67: PLUS_DI / MINUS_DI / PLUS_DM / MINUS_DM / DX ──
        self.render_research_round67_windows(ctx);

        // ── Research Round 68 windows ──
        self.render_research_round68_windows(ctx);

        // ── Research Round 71 windows ──
        self.render_research_round71_windows(ctx);

        // ── Research Round 72 CDL* windows ─────────────────────────────────
        self.render_research_round72_windows(ctx);

        // ── Research Round 77 popup windows ──
        self.render_research_round77_windows(ctx);

        // ── Research Round 78 popup windows ──
        self.render_research_round78_windows(ctx);

        // ── Research Round 76 (Quant Stats) popup windows ──
        self.render_research_round76_windows(ctx);

        // Research ingest and packet viewer
        self.render_research_ingest_windows(ctx);

        // GY — Treasury Yield Curve
        // Macro data windows
        self.render_macro_windows(ctx);
        // SEC, macro calendar, earnings, and congressional-trade windows
        self.render_sec_calendar_windows(ctx);

        // SwapHarvest, Darwinex Radar, scrape status, and earnings windows
        self.render_scrape_darwinia_windows(ctx);

        // Market analytics, calendars, screeners, and portfolio risk windows
        self.render_market_analytics_windows(ctx);

        // Order flow, bookmap, orderbook DOM, and indicator compiler windows
        self.render_trading_tools_windows(ctx);

        // Risk tools, alerts, outlier scanner, trade journal, and margin windows
        self.render_risk_journal_windows(ctx);

        // Cache stats, storage manager, and LAN sync windows
        self.render_storage_sync_windows(ctx);

        // Object list, command reference, and data-window overlays
        self.render_workspace_reference_windows(ctx);

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
