use super::*;

impl TyphooNApp {
    /// Render the Settings window. Returns `true` when a text-edit loss of focus
    /// or folder-picker assignment inside the window flagged the session for
    /// persistence. The caller is responsible for triggering `save_session()` —
    /// the method cannot inline the save because the closure already borrows
    /// `self` mutably.
    pub(super) fn render_settings_window(&mut self, ctx: &egui::Context) -> bool {
        let mut settings_save_after = false;
        if !self.show_settings {
            return settings_save_after;
        }
        let mut show_settings = self.show_settings;
        egui::Window::new("Settings")
            .open(&mut show_settings)
            .resizable(true).default_size([450.0, 500.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                // ── API Keys (matching old WebKit connection modal) ──
                ui.heading("API Keys");
                ui.separator();
                egui::Grid::new("api_keys_settings").num_columns(2).spacing(egui::vec2(8.0, 4.0)).show(ui, |ui| {
                    // Broker key rows are gated on the matching `<broker>_enabled` flag
                    // so disabled brokers don't clutter Settings with credentials the
                    // app won't use. Toggle the broker on under "Broker modules" below
                    // to expose its fields.
                    if self.alpaca_enabled {
                        ui.label("Alpaca API Key:");
                        ui.add(egui::TextEdit::singleline(&mut self.broker_api_key).desired_width(250.0).password(true));
                        ui.end_row();
                        ui.label("Alpaca Secret:");
                        ui.add(egui::TextEdit::singleline(&mut self.broker_secret).desired_width(250.0).password(true));
                        ui.end_row();
                        ui.label("Alpaca Mode:");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.broker_paper, true, "Paper");
                            ui.radio_value(&mut self.broker_paper, false, "Live");
                        });
                        ui.end_row();
                    }
                    ui.label("Finnhub API Key:");
                    ui.add(egui::TextEdit::singleline(&mut self.finnhub_key).desired_width(250.0).password(true));
                    ui.end_row();
                    ui.label("FRED API Key:");
                    ui.add(egui::TextEdit::singleline(&mut self.fred_key).desired_width(250.0).password(true));
                    ui.end_row();
                    if self.tastytrade_enabled {
                        ui.label("tastytrade User:");
                        ui.add(egui::TextEdit::singleline(&mut self.tt_username).desired_width(250.0));
                        ui.end_row();
                        ui.label("tastytrade Pass:");
                        ui.add(egui::TextEdit::singleline(&mut self.tt_password).desired_width(250.0).password(true));
                        ui.end_row();
                        ui.label("tastytrade Mode:");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.tt_sandbox, true, "Sandbox");
                            ui.radio_value(&mut self.tt_sandbox, false, "Production");
                        });
                        ui.end_row();
                    }
                    if self.kraken_enabled {
                        ui.label("Kraken REST API Key:");
                        ui.add(egui::TextEdit::singleline(&mut self.kraken_api_key).desired_width(250.0).password(true));
                        ui.end_row();
                        ui.label("Kraken REST API Secret:");
                        ui.add(egui::TextEdit::singleline(&mut self.kraken_api_secret).desired_width(250.0).password(true));
                        ui.end_row();
                        ui.label("Kraken WS API Key:");
                        ui.add(egui::TextEdit::singleline(&mut self.kraken_ws_api_key).desired_width(250.0).password(true));
                        ui.end_row();
                        ui.label("Kraken WS API Secret:");
                        ui.add(egui::TextEdit::singleline(&mut self.kraken_ws_api_secret).desired_width(250.0).password(true));
                        ui.end_row();
                    }
                    ui.label("Gemini API Key:");
                    ui.add(egui::TextEdit::singleline(&mut self.gemini_key).desired_width(250.0).password(true));
                    ui.end_row();
                    ui.label("Grok (xAI) Key:");
                    ui.add(egui::TextEdit::singleline(&mut self.xai_key).desired_width(250.0).password(true));
                    ui.end_row();
                    ui.label("Mistral API Key:");
                    ui.add(egui::TextEdit::singleline(&mut self.mistral_key).desired_width(250.0).password(true));
                    ui.end_row();
                    ui.label("Perplexity Key:");
                    ui.add(egui::TextEdit::singleline(&mut self.perplexity_key).desired_width(250.0).password(true));
                    ui.end_row();
                    ui.label("Matrix Token:");
                    ui.add(egui::TextEdit::singleline(&mut self.matrix_access_token).desired_width(250.0).password(true));
                    ui.end_row();
                    ui.label("Matrix User ID:");
                    ui.add(egui::TextEdit::singleline(&mut self.matrix_user_id).desired_width(250.0).hint_text("@user:matrix.org"));
                    ui.end_row();
                });
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Broker modules").color(AXIS_TEXT).small().strong());
                ui.horizontal_wrapped(|ui| {
                    let alpaca_before = self.alpaca_enabled;
                    if ui.checkbox(&mut self.alpaca_enabled, "Enable Alpaca").on_hover_text("When off: no startup login, no account/position/order requests, no Alpaca bar sync, and Alpaca order buttons stay inactive. Stored bar data is left untouched.").changed() {
                        settings_save_after = true;
                        if alpaca_before && !self.alpaca_enabled {
                            self.broker_connected = false;
                            self.live_account = None;
                            self.live_positions.clear();
                            self.live_orders.clear();
                            self.pending_alpaca_fetches.clear();
                            self.log.push_back(LogEntry::info("Alpaca disabled — stopped UI-side login/sync/position/order activity. Existing cache data was not deleted."));
                        }
                    }
                    if ui.checkbox(&mut self.darwinex_enabled, "Enable Darwinex").on_hover_text("When off: hides Darwinex from the Scope cycle and disables Darwinex-scoped discovery. Stored/imported data is left untouched.").changed() {
                        settings_save_after = true;
                        if !self.darwinex_enabled && self.broker_scope == EventSource::Darwinex {
                            self.broker_scope = EventSource::All;
                        }
                    }
                    let tt_before = self.tastytrade_enabled;
                    if ui.checkbox(&mut self.tastytrade_enabled, "Enable tastytrade").on_hover_text("When off: no manual/startup connection, positions/balances, or tastytrade bar sync. Stored data is left untouched.").changed() {
                        settings_save_after = true;
                        if tt_before && !self.tastytrade_enabled {
                            self.tt_connected = false;
                            self.tt_positions.clear();
                            self.tt_balances = None;
                            self.pending_tastytrade_fetches.clear();
                            self.log.push_back(LogEntry::info("tastytrade disabled — stopped UI-side broker activity. Existing cache data was not deleted."));
                        }
                    }
                    let kr_before = self.kraken_enabled;
                    if ui.checkbox(&mut self.kraken_enabled, "Enable Kraken").on_hover_text("When off: no startup login, private REST/WS account sync, Kraken Spot/Equities/Futures bar sync, or Kraken order buttons. Stored data is left untouched.").changed() {
                        settings_save_after = true;
                        if kr_before && !self.kraken_enabled {
                            self.kraken_connected = false;
                            self.kr_positions.clear();
                            self.kraken_balances.clear();
                            self.kraken_open_orders.clear();
                            self.pending_kraken_fetches.clear();
                            self.pending_kraken_futures_fetches.clear();
                            self.log.push_back(LogEntry::info("Kraken disabled — stopped UI-side login/sync/position/order activity. Existing cache data was not deleted."));
                        }
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let connect_label = if self.broker_connected {
                        egui::RichText::new("Alpaca Connected").color(UP)
                    } else {
                        egui::RichText::new("Connect Alpaca")
                    };
                    if ui.add_enabled(self.alpaca_enabled, egui::Button::new(connect_label)).clicked() && !self.broker_connected {
                        if !self.broker_api_key.is_empty() && !self.broker_secret.is_empty() {
                            // Save all credentials to system keyring
                            if let Err(e) = keyring::store(keyring::keys::ALPACA_API_KEY, &self.broker_api_key) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store alpaca_api_key failed: {}", e)));
                            }
                            if let Err(e) = keyring::store(keyring::keys::ALPACA_SECRET, &self.broker_secret) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store alpaca_secret failed: {}", e)));
                            }
                            if !self.finnhub_key.is_empty() {
                                if let Err(e) = keyring::store(keyring::keys::FINNHUB_KEY, &self.finnhub_key) {
                                    self.log.push_back(LogEntry::warn(format!("Keyring store finnhub_key failed: {}", e)));
                                }
                            }
                            if !self.fred_key.is_empty() {
                                if let Err(e) = keyring::store(keyring::keys::FRED_KEY, &self.fred_key) {
                                    self.log.push_back(LogEntry::warn(format!("Keyring store fred_key failed: {}", e)));
                                }
                            }
                            if !self.tt_username.is_empty() {
                                if let Err(e) = keyring::store(keyring::keys::TT_USERNAME, &self.tt_username) {
                                    self.log.push_back(LogEntry::warn(format!("Keyring store tt_username failed: {}", e)));
                                }
                                if let Err(e) = keyring::store(keyring::keys::TT_PASSWORD, &self.tt_password) {
                                    self.log.push_back(LogEntry::warn(format!("Keyring store tt_password failed: {}", e)));
                                }
                            }
                            self.log.push_back(LogEntry::info("Credentials saved to system keyring"));
                            let capacity = self.alpaca_sync_capacity();
                            let _ = self.broker_tx.send(BrokerCmd::Connect {
                                api_key: self.broker_api_key.clone(),
                                secret: self.broker_secret.clone(),
                                paper: self.broker_paper,
                                bar_requests_per_minute: self.alpaca_effective_historical_rpm(),
                                fetch_permits: capacity.fetch_permits,
                            });
                        }
                    }
                    // tastytrade connect button — quotes + metrics via REST, no historical bars
                    if !self.tt_username.is_empty() && !self.tt_password.is_empty() {
                        let tt_label = if self.tt_connected {
                            egui::RichText::new("tastytrade Connected").color(UP)
                        } else {
                            egui::RichText::new("Connect tastytrade")
                        };
                        if ui.add_enabled(self.tastytrade_enabled, egui::Button::new(tt_label)).clicked() && !self.tt_connected {
                            if let Err(e) = keyring::store(keyring::keys::TT_USERNAME, &self.tt_username) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store tt_username failed: {}", e)));
                            }
                            if let Err(e) = keyring::store(keyring::keys::TT_PASSWORD, &self.tt_password) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store tt_password failed: {}", e)));
                            }
                            let _ = self.broker_tx.send(BrokerCmd::TastytradeConnect {
                                username: self.tt_username.clone(),
                                password: self.tt_password.clone(),
                                sandbox: self.tt_sandbox,
                            });
                            self.log.push_back(LogEntry::info(format!(
                                "tastytrade {} — connecting...",
                                if self.tt_sandbox { "Sandbox" } else { "Production" }
                            )));
                        }
                    }
                    // Kraken connect button
                    if (!self.kraken_api_key.is_empty() && !self.kraken_api_secret.is_empty())
                        || (!self.kraken_ws_api_key.is_empty()
                            && !self.kraken_ws_api_secret.is_empty())
                    {
                        let kraken_label = if self.kraken_connected {
                            egui::RichText::new("Kraken Connected").color(UP)
                        } else {
                            egui::RichText::new("Connect Kraken")
                        };
                        if ui.add_enabled(self.kraken_enabled, egui::Button::new(kraken_label)).clicked() && !self.kraken_connected {
                            if let Err(e) = keyring::store(keyring::keys::KRAKEN_API_KEY, &self.kraken_api_key) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store kraken_api_key failed: {}", e)));
                            }
                            if let Err(e) = keyring::store(keyring::keys::KRAKEN_API_SECRET, &self.kraken_api_secret) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store kraken_api_secret failed: {}", e)));
                            }
                            if let Err(e) = keyring::store(keyring::keys::KRAKEN_WS_API_KEY, &self.kraken_ws_api_key) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store kraken_ws_api_key failed: {}", e)));
                            }
                            if let Err(e) = keyring::store(keyring::keys::KRAKEN_WS_API_SECRET, &self.kraken_ws_api_secret) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store kraken_ws_api_secret failed: {}", e)));
                            }
                            let _ = self.broker_tx.send(BrokerCmd::KrakenConnect {
                                api_key: self.kraken_api_key.clone(),
                                api_secret: self.kraken_api_secret.clone(),
                                ws_api_key: self.kraken_ws_api_key.clone(),
                                ws_api_secret: self.kraken_ws_api_secret.clone(),
                            });
                            self.log.push_back(LogEntry::info("Kraken — connecting..."));
                        }
                    }
                });

                // Matrix chat — save token to keyring
                ui.horizontal(|ui| {
                    if !self.matrix_access_token.is_empty() && self.matrix_access_token != "none" {
                        let matrix_label = if !self.matrix_user_id.is_empty() {
                            egui::RichText::new(format!("Matrix: {}", self.matrix_user_id)).color(UP)
                        } else {
                            egui::RichText::new("Save Matrix Token")
                        };
                        if ui.button(matrix_label).clicked() {
                            let _ = keyring::store(keyring::keys::MATRIX_ACCESS_TOKEN, &self.matrix_access_token);
                            if !self.matrix_user_id.is_empty() {
                                let _ = keyring::store(keyring::keys::MATRIX_USER_ID, &self.matrix_user_id);
                            }
                            self.log.push_back(LogEntry::info("Matrix token saved to keyring"));
                            // Join room + fetch messages
                            let _ = self.broker_tx.send(BrokerCmd::MatrixJoinRoom {
                                room_id: self.matrix_room.clone(),
                                access_token: self.matrix_access_token.clone(),
                            });
                            let _ = self.broker_tx.send(BrokerCmd::MatrixFetchMessages {
                                room_id: self.matrix_room.clone(),
                                access_token: self.matrix_access_token.clone(),
                            });
                        }
                    }
                });

                ui.add_space(10.0);
                ui.heading("General");
                ui.separator();
                ui.label("Theme: OLED Dark (#000000)");
                ui.label("Font: Monospace 11px (Consolas equiv.)");
                ui.label("Refresh rate: 250ms");
                ui.label("Chart default: 200 visible bars");

                ui.add_space(10.0);
                ui.heading("Fundamentals Symbol Sources");
                ui.separator();
                ui.label(egui::RichText::new("Select which brokers to pull stock tickers from for Yahoo fundamentals scrape.").color(AXIS_TEXT).small());
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.fund_source_mt5, "MT5 / Darwinex");
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

                ui.add_space(10.0);
                ui.heading("Data Sources");
                ui.separator();
                ui.label("SQLite cache: ~/.config/typhoon-terminal/cache/typhoon_cache.db");
                if let Some((rows, kv, size)) = self.bg.cache_stats {
                    ui.label(format!("Bar entries: {}  |  KV entries: {}  |  DB size: {} KB", rows, kv, size / 1024));
                }
                if self.alpaca_enabled {
                    let alpaca_status = if self.broker_connected { "Connected" } else { "Disconnected" };
                    ui.label(format!("Alpaca: REST API + WebSocket — {}", alpaca_status));
                }
                if self.tastytrade_enabled {
                    let tt_status = if self.tt_connected { "Connected" } else { "Disconnected" };
                    ui.label(format!("tastytrade: REST API — {}", tt_status));
                }
                if self.kraken_enabled {
                    let kraken_status = if self.kraken_connected { "Connected" } else { "Disconnected" };
                    ui.label(format!("Kraken: REST API + WebSocket — {}", kraken_status));
                }
                if self.kraken_enabled {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Kraken automated scrape universes")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    let mut kraken_scrape_changed = false;
                    ui.horizontal_wrapped(|ui| {
                        kraken_scrape_changed |= ui
                            .checkbox(&mut self.kraken_scrape_xstocks, "xStocks / ETFs")
                            .on_hover_text("Tokenized stocks and ETFs from Kraken Spot/xStocks.")
                            .changed();
                        kraken_scrape_changed |= ui
                            .checkbox(&mut self.kraken_scrape_crypto_crosses, "Crypto crosses")
                            .on_hover_text("Non-fiat crypto crosses such as ETH/BTC.")
                            .changed();
                        kraken_scrape_changed |= ui
                            .checkbox(&mut self.kraken_scrape_futures, "Futures")
                            .on_hover_text("Kraken Futures public instruments and candles.")
                            .changed();
                    });
                    ui.label(egui::RichText::new("Global crypto/fiat quote filters").color(AXIS_TEXT).small());
                    ui.horizontal_wrapped(|ui| {
                        kraken_scrape_changed |= ui.checkbox(&mut self.crypto_fiat_quote_usd, "USD").changed();
                        kraken_scrape_changed |= ui.checkbox(&mut self.crypto_fiat_quote_usdt, "USDT").changed();
                        kraken_scrape_changed |= ui.checkbox(&mut self.crypto_fiat_quote_usdc, "USDC").changed();
                        kraken_scrape_changed |= ui.checkbox(&mut self.crypto_fiat_quote_usdg, "USDG").changed();
                        kraken_scrape_changed |= ui.checkbox(&mut self.crypto_fiat_quote_eur, "EUR").changed();
                        kraken_scrape_changed |= ui.checkbox(&mut self.crypto_fiat_quote_gbp, "GBP").changed();
                        kraken_scrape_changed |= ui.checkbox(&mut self.crypto_fiat_quote_cad, "CAD").changed();
                        kraken_scrape_changed |= ui.checkbox(&mut self.crypto_fiat_quote_aud, "AUD").changed();
                        kraken_scrape_changed |= ui.checkbox(&mut self.crypto_fiat_quote_jpy, "JPY").changed();
                        kraken_scrape_changed |= ui.checkbox(&mut self.crypto_fiat_quote_chf, "CHF").changed();
                    });
                    if kraken_scrape_changed {
                        self.kraken_scrape_usd_crypto = self.crypto_fiat_quote_usd
                            || self.crypto_fiat_quote_usdt
                            || self.crypto_fiat_quote_usdc
                            || self.crypto_fiat_quote_usdg;
                        self.kraken_scrape_fiat_crypto = self.crypto_fiat_quote_eur
                            || self.crypto_fiat_quote_gbp
                            || self.crypto_fiat_quote_cad
                            || self.crypto_fiat_quote_aud
                            || self.crypto_fiat_quote_jpy
                            || self.crypto_fiat_quote_chf;
                        settings_save_after = true;
                        self.pending_kraken_fetches.clear();
                        if !self.kraken_scrape_futures {
                            self.pending_kraken_futures_fetches.clear();
                        }
                    }
                    ui.label(
                        egui::RichText::new(
                            "These category and quote filters control automated broker public scraping. Kraken uses them now; future crypto brokers should call the same quote filter instead of adding broker-specific assumptions.",
                        )
                        .color(AXIS_TEXT)
                        .small(),
                    );
                }
                ui.label("Finnhub: News, Analyst, Insider Sentiment, Short Interest");
                ui.label("SEC EDGAR: Filing scraper + Form 4 insider trades");
                ui.add_space(6.0);
                ui.separator();
                // LAN Client mode toggle
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.lan_client_enabled, "LAN Client Mode");
                    if self.lan_client_enabled {
                        ui.label(egui::RichText::new("(all local syncing disabled — data from server)").color(AXIS_TEXT).small());
                    }
                });
                if self.lan_client_enabled {
                    ui.horizontal(|ui| {
                        ui.label("Server IP:");
                        ui.add(egui::TextEdit::singleline(&mut self.lan_server_ip).desired_width(140.0).hint_text("192.168.1.100").font(egui::TextStyle::Monospace));
                        ui.label("Port:");
                        ui.add(egui::TextEdit::singleline(&mut self.lan_sync_port).desired_width(50.0).font(egui::TextStyle::Monospace));
                    });
                }

                ui.add_space(10.0);
                ui.heading("MT5 BarCacheWriter Sources");
                ui.separator();
                ui.label(egui::RichText::new("Paths to typhoon_mt5_cache.db files written by BarCacheWriter EA.").color(AXIS_TEXT).small());
                ui.label(egui::RichText::new("Data is synced into the main cache on startup and on demand.").color(AXIS_TEXT).small());
                let now_s = chrono::Utc::now().timestamp();
                for i in 0..4 {
                    ui.horizontal(|ui| {
                        ui.label(format!("MT5 #{}:", i + 1));
                        let r = ui.add(egui::TextEdit::singleline(&mut self.mt5_db_paths[i]).desired_width(400.0).hint_text("/path/to/typhoon_mt5_cache.db"));
                        if r.lost_focus() { settings_save_after = true; }
                        // Show status indicator + heartbeat freshness so the user
                        // can tell "file exists but BarCacheWriter is dead" apart
                        // from "actively writing". BCW's heartbeat cadence is
                        // ~30s, so > 90s without a fresh beat means something
                        // (EA crash, MT5 disconnect, frozen terminal) has stopped
                        // the writer even though the DB file is still on disk.
                        if !self.mt5_db_paths[i].is_empty() {
                            let exists = std::path::Path::new(&self.mt5_db_paths[i]).exists();
                            let (icon, col) = if exists { ("\u{25CF}", UP) } else { ("\u{25CF}", DOWN) };
                            ui.label(egui::RichText::new(icon).color(col));
                            if exists {
                                let beat = self.mt5_heartbeats.iter()
                                    .find(|h| h.0 == self.mt5_db_paths[i]);
                                match beat {
                                    Some(h) => {
                                        let age = now_s.saturating_sub(h.3);
                                        let (txt, col) = if age <= 45 {
                                            (format!("beat {}s ago", age), UP)
                                        } else if age <= 90 {
                                            (format!("beat {}s ago (lagging)", age),
                                             egui::Color32::from_rgb(220, 200, 80))
                                        } else {
                                            (format!("STALE ({}s)", age), DOWN)
                                        };
                                        ui.label(egui::RichText::new(txt).color(col).small());
                                    }
                                    None => {
                                        ui.label(egui::RichText::new("no heartbeat yet")
                                            .color(AXIS_TEXT).small());
                                    }
                                }
                            }
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button("Sync MT5 Data Now").clicked() {
                        let paths: Vec<String> = self.mt5_db_paths.iter()
                            .filter(|p| !p.is_empty() && std::path::Path::new(p.as_str()).exists())
                            .cloned().collect();
                        if paths.is_empty() {
                            self.log.push_back(LogEntry::warn("No valid MT5 database paths configured"));
                        } else {
                            let count = paths.len();
                            let _ = self.broker_tx.send(BrokerCmd::Mt5Sync {
                                sources: paths,
                                enabled_timeframes: self.enabled_standard_sync_timeframes(),
                            });
                            self.log.push_back(LogEntry::info(format!("MT5 sync started ({} sources)...", count)));
                        }
                    }
                    ui.checkbox(&mut self.mt5_auto_sync, "Auto-sync every 30s")
                        .on_hover_text("Matches BarCacheWriter's 30s write cadence. Silent — no log spam.");
                });

                if self.darwinex_enabled {
                ui.add_space(10.0);
                ui.heading("Darwinex");
                ui.separator();
                ui.label("Correlation limit: 0.95 / 45d");
                ui.label("Margin accounts: 100%");
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label("FTP Dir:");
                    let r = ui.add(egui::TextEdit::singleline(&mut self.darwin_ftp_dir).desired_width(300.0).hint_text("/path/to/darwinex/ftp"));
                    if r.lost_focus() { settings_save_after = true; }
                    if ui.button("Browse").clicked() {
                        if let Some(dir) = rfd::FileDialog::new()
                            .set_title("Select Darwinex FTP Directory")
                            .pick_folder()
                        {
                            self.darwin_ftp_dir = dir.display().to_string();
                            if let Ok(mut d) = self.shared_ftp_dir.lock() { *d = self.darwin_ftp_dir.clone(); }
                            settings_save_after = true;
                        }
                    }
                    if !self.darwin_ftp_dir.is_empty() {
                        let exists = std::path::Path::new(&self.darwin_ftp_dir).is_dir();
                        let (icon, col) = if exists { ("\u{25CF}", UP) } else { ("\u{25CF}", DOWN) };
                        ui.label(egui::RichText::new(icon).color(col));
                    }
                });
                ui.label(egui::RichText::new("Contains DARWIN CSV data for FTP scanner, D-Score, investor flow, price series.").color(AXIS_TEXT).small());
                ui.horizontal(|ui| {
                    if ui.button("Import ExportSymbols CSV").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("CSV", &["csv"])
                            .set_title("Import ExportSymbols.mq5 CSV")
                            .pick_file()
                        {
                            match std::fs::read_to_string(&path) {
                                Ok(csv) => {
                                    let normalized = darwin::normalize_imported_symbols_csv(&csv);
                                    let imported = normalized.lines().count();
                                    if imported == 0 {
                                        self.log.push_back(LogEntry::err(format!(
                                            "Import failed: {} is not a valid ExportSymbols CSV",
                                            path.display()
                                        )));
                                    } else if let Some(ref cache) = self.cache {
                                        match cache.put_kv("darwin:imported_symbols_csv", &csv) {
                                            Ok(()) => {
                                                if let Some(conn) = cache.try_connection() {
                                                    self.darwinex_radar_data =
                                                        darwin::load_all_specs_parsed(&conn)
                                                            .unwrap_or_default();
                                                }
                                                self.log.push_back(LogEntry::info(format!(
                                                    "Imported Darwin symbol universe from {} ({} symbols)",
                                                    path.display(),
                                                    imported
                                                )));
                                            }
                                            Err(e) => self.log.push_back(LogEntry::err(
                                                format!("Import failed: {}", e),
                                            )),
                                        }
                                    } else {
                                        self.log.push_back(LogEntry::warn(
                                            "Cache not ready — cannot import symbols yet",
                                        ));
                                    }
                                }
                                Err(e) => self.log.push_back(LogEntry::err(format!(
                                    "Import failed: {}",
                                    e
                                ))),
                            }
                        }
                    }
                    ui.label(
                        egui::RichText::new(
                            "Use ExportSymbols.mq5 to define Darwin tradable symbols without MT5 bar sync.",
                        )
                        .color(AXIS_TEXT)
                        .small(),
                    );
                });

                ui.add_space(10.0);
                ui.heading("DARWIN XLSX Import");
                ui.separator();
                ui.label(egui::RichText::new("Directory containing MT5 XLSX trade history files (any server — Darwinex, Axion, etc.)").color(AXIS_TEXT).small());
                ui.horizontal(|ui| {
                    ui.label("XLSX Dir:");
                    let r = ui.add(egui::TextEdit::singleline(&mut self.darwin_xlsx_dir).desired_width(250.0).hint_text("/path/to/darwin/xlsx"));
                    if r.lost_focus() { settings_save_after = true; }
                    if ui.button("Browse").clicked() {
                        if let Some(dir) = rfd::FileDialog::new()
                            .set_title("Select MT5 XLSX Trade History Directory")
                            .pick_folder()
                        {
                            self.darwin_xlsx_dir = dir.display().to_string();
                            settings_save_after = true;
                        }
                    }
                });
                if !self.darwin_xlsx_dir.is_empty() {
                    if ui.button("Import All XLSX Now").clicked() {
                        let db_path = cache_db_path();
                        let _ = self.broker_tx.send(BrokerCmd::DarwinImportAll {
                            dir: PathBuf::from(&self.darwin_xlsx_dir),
                            db_path,
                        });
                        self.log.push_back(LogEntry::info(format!("DARWIN XLSX import started from {}", self.darwin_xlsx_dir)));
                    }
                }
                ui.label("Margin accounts: 100%");
                } // end darwinex_enabled gate

                ui.add_space(10.0);
                ui.heading("Notifications");
                ui.separator();
                ui.label(egui::RichText::new("Alerts trigger push notifications when configured.").color(AXIS_TEXT).small());
                egui::Grid::new("notif_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Discord Webhook:");
                    ui.add(egui::TextEdit::singleline(&mut self.discord_webhook).desired_width(300.0).password(true));
                    ui.end_row();
                    ui.label("Pushover Token:");
                    ui.add(egui::TextEdit::singleline(&mut self.pushover_token).desired_width(200.0).password(true));
                    ui.end_row();
                    ui.label("Pushover User:");
                    ui.add(egui::TextEdit::singleline(&mut self.pushover_user).desired_width(200.0).password(true));
                    ui.end_row();
                    ui.label("ntfy Topic:");
                    ui.add(egui::TextEdit::singleline(&mut self.ntfy_topic).desired_width(200.0));
                    ui.end_row();
                    ui.label("Anthropic API Key:");
                    ui.add(egui::TextEdit::singleline(&mut self.anthropic_key).desired_width(250.0).password(true));
                    ui.end_row();
                    ui.label("OpenAI API Key:");
                    ui.add(egui::TextEdit::singleline(&mut self.openai_key).desired_width(250.0).password(true));
                    ui.end_row();
                });
                if ui.small_button("Test Notification").clicked() {
                    let _ = self.broker_tx.send(BrokerCmd::SendNotification {
                        discord_webhook: self.discord_webhook.clone(),
                        pushover_token: self.pushover_token.clone(),
                        pushover_user: self.pushover_user.clone(),
                        ntfy_topic: self.ntfy_topic.clone(),
                        message: "TyphooN Terminal: test notification".into(),
                    });
                }

                ui.add_space(10.0);
                if ui.button("Open Indicators Panel").clicked() {
                    self.show_indicators_panel = true;
                }
                if ui.button("Storage Manager").clicked() {
                    self.show_storage = true;
                }
                });
            });
        self.show_settings = show_settings;
        settings_save_after
    }
}
