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
        // Dynamic account slots (ADR-130): the grid closure below borrows `self`,
        // so add/remove clicks are recorded here and applied after it returns.
        let mut alpaca_add_account = false;
        let mut alpaca_remove_account: Option<usize> = None;
        let mut kraken_add_account = false;
        let mut kraken_remove_account: Option<usize> = None;
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
                        // Alpaca account slots 1–4 (ADR-130) render identically:
                        // Key / Secret / Paper-or-Live. Every configured slot
                        // joins the bar-sync rotation and can trade; TradeCopy
                        // targets are picked in the TradeCopy window (TRADECOPY).
                        // Credentials persist to the keyring on field edit —
                        // no Connect click required.
                        {
                            let key_resp = {
                                ui.label("Alpaca #1 Key:");
                                let r = ui.add(egui::TextEdit::singleline(&mut self.broker_api_key).desired_width(250.0).password(true));
                                ui.end_row();
                                r
                            };
                            if key_resp.lost_focus() {
                                self.persist_credential_async(keyring::keys::ALPACA_API_KEY.into(), self.broker_api_key.clone());
                                settings_save_after = true;
                            }
                            let secret_resp = {
                                ui.label("Alpaca #1 Secret:");
                                let r = ui.add(egui::TextEdit::singleline(&mut self.broker_secret).desired_width(250.0).password(true));
                                ui.end_row();
                                r
                            };
                            if secret_resp.lost_focus() {
                                self.persist_credential_async(keyring::keys::ALPACA_SECRET.into(), self.broker_secret.clone());
                                settings_save_after = true;
                            }
                            ui.label("Alpaca #1 Mode:");
                            ui.horizontal(|ui| {
                                let a = ui.radio_value(&mut self.broker_paper, true, "Paper").changed();
                                let b = ui.radio_value(&mut self.broker_paper, false, "Live").changed();
                                if a || b {
                                    settings_save_after = true;
                                }
                            });
                            ui.end_row();
                        }
                        for idx in 0..self.alpaca_extra_accounts.len() {
                            let slot = idx + 2;
                            let (key_name, secret_name) = super::broker_accounts::alpaca_slot_keyring_keys(slot);
                            ui.label(format!("Alpaca #{slot} Key:"));
                            let key_resp = ui.add(egui::TextEdit::singleline(&mut self.alpaca_extra_accounts[idx].api_key).desired_width(250.0).password(true));
                            ui.end_row();
                            if key_resp.lost_focus() {
                                self.persist_credential_async(key_name, self.alpaca_extra_accounts[idx].api_key.clone());
                                settings_save_after = true;
                            }
                            ui.label(format!("Alpaca #{slot} Secret:"));
                            let secret_resp = ui.add(egui::TextEdit::singleline(&mut self.alpaca_extra_accounts[idx].secret).desired_width(250.0).password(true));
                            ui.end_row();
                            if secret_resp.lost_focus() {
                                self.persist_credential_async(secret_name, self.alpaca_extra_accounts[idx].secret.clone());
                                settings_save_after = true;
                            }
                            ui.label(format!("Alpaca #{slot} Mode:"));
                            ui.horizontal(|ui| {
                                let a = ui.radio_value(&mut self.alpaca_extra_accounts[idx].paper, true, "Paper").changed();
                                let b = ui.radio_value(&mut self.alpaca_extra_accounts[idx].paper, false, "Live").changed();
                                if a || b {
                                    settings_save_after = true;
                                }
                                if ui.button("✕ Remove").on_hover_text("Remove this account slot").clicked() {
                                    alpaca_remove_account = Some(idx);
                                }
                            });
                            ui.end_row();
                        }
                        if super::broker_accounts::can_add_account_slot(self.alpaca_extra_accounts.len()) {
                            ui.label("");
                            if ui.button("➕ Add Alpaca account").clicked() {
                                alpaca_add_account = true;
                            }
                            ui.end_row();
                        }
                    }
                    ui.label("Finnhub API Key:");
                    ui.add(egui::TextEdit::singleline(&mut self.finnhub_key).desired_width(250.0).password(true));
                    ui.end_row();
                    ui.label("FRED API Key:");
                    ui.add(egui::TextEdit::singleline(&mut self.fred_key).desired_width(250.0).password(true));
                    ui.end_row();
                    ui.label("CryptoPanic Token:");
                    ui.add(egui::TextEdit::singleline(&mut self.cryptopanic_key).desired_width(250.0).password(true).hint_text("free at cryptopanic.com → API"));
                    ui.end_row();
                    if self.kraken_enabled {
                        // Kraken slot 1 keeps separate REST + WS credential
                        // pairs; extra slots 2–4 are uniform Key/Secret trading
                        // identities for primary cycling (Kraken market data is
                        // public, so extra accounts don't join bar sync). All
                        // fields persist to the keyring on edit.
                        let mut kraken_main_edits: Vec<(&'static str, String)> = Vec::new();
                        ui.label("Kraken REST API Key:");
                        if ui.add(egui::TextEdit::singleline(&mut self.kraken_api_key).desired_width(250.0).password(true)).lost_focus() {
                            kraken_main_edits.push((keyring::keys::KRAKEN_API_KEY, self.kraken_api_key.clone()));
                        }
                        ui.end_row();
                        ui.label("Kraken REST API Secret:");
                        if ui.add(egui::TextEdit::singleline(&mut self.kraken_api_secret).desired_width(250.0).password(true)).lost_focus() {
                            kraken_main_edits.push((keyring::keys::KRAKEN_API_SECRET, self.kraken_api_secret.clone()));
                        }
                        ui.end_row();
                        ui.label("Kraken WS API Key:");
                        if ui.add(egui::TextEdit::singleline(&mut self.kraken_ws_api_key).desired_width(250.0).password(true)).lost_focus() {
                            kraken_main_edits.push((keyring::keys::KRAKEN_WS_API_KEY, self.kraken_ws_api_key.clone()));
                        }
                        ui.end_row();
                        ui.label("Kraken WS API Secret:");
                        if ui.add(egui::TextEdit::singleline(&mut self.kraken_ws_api_secret).desired_width(250.0).password(true)).lost_focus() {
                            kraken_main_edits.push((keyring::keys::KRAKEN_WS_API_SECRET, self.kraken_ws_api_secret.clone()));
                        }
                        ui.end_row();
                        ui.label("Kraken Mode:");
                        ui.horizontal(|ui| {
                            let a = ui.radio_value(&mut self.kraken_paper, true, "Paper").changed();
                            let b = ui.radio_value(&mut self.kraken_paper, false, "Live").changed();
                            if a || b {
                                settings_save_after = true;
                            }
                        });
                        ui.end_row();
                        for (key_name, value) in kraken_main_edits {
                            self.persist_credential_async(key_name.into(), value);
                            settings_save_after = true;
                        }
                        for idx in 0..self.kraken_extra_accounts.len() {
                            let slot = idx + 2;
                            let (key_name, secret_name) = super::broker_accounts::kraken_slot_keyring_keys(slot);
                            ui.label(format!("Kraken #{slot} Key:"));
                            let key_resp = ui.add(egui::TextEdit::singleline(&mut self.kraken_extra_accounts[idx].api_key).desired_width(250.0).password(true));
                            ui.end_row();
                            if key_resp.lost_focus() {
                                self.persist_credential_async(key_name, self.kraken_extra_accounts[idx].api_key.clone());
                                settings_save_after = true;
                            }
                            ui.label(format!("Kraken #{slot} Secret:"));
                            let secret_resp = ui.add(egui::TextEdit::singleline(&mut self.kraken_extra_accounts[idx].secret).desired_width(250.0).password(true));
                            ui.end_row();
                            if secret_resp.lost_focus() {
                                self.persist_credential_async(secret_name, self.kraken_extra_accounts[idx].secret.clone());
                                settings_save_after = true;
                            }
                            ui.label(format!("Kraken #{slot} Mode:"));
                            ui.horizontal(|ui| {
                                let a = ui.radio_value(&mut self.kraken_extra_accounts[idx].paper, true, "Paper").changed();
                                let b = ui.radio_value(&mut self.kraken_extra_accounts[idx].paper, false, "Live").changed();
                                if a || b {
                                    settings_save_after = true;
                                }
                                if ui.button("✕ Remove").on_hover_text("Remove this account slot").clicked() {
                                    kraken_remove_account = Some(idx);
                                }
                            });
                            ui.end_row();
                        }
                        if super::broker_accounts::can_add_account_slot(self.kraken_extra_accounts.len()) {
                            ui.label("");
                            if ui.button("➕ Add Kraken account").clicked() {
                                kraken_add_account = true;
                            }
                            ui.end_row();
                        }
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
                    if ui.checkbox(&mut self.alpaca_enabled, "Enable Alpaca").on_hover_text("When off: no startup login, account/position/order requests, targeted Alpaca fallback, or Alpaca order buttons. Stored bar data is left untouched. Broad Alpaca universe bar sync is controlled separately below.").changed() {
                        settings_save_after = true;
                        if alpaca_before && !self.alpaca_enabled {
                            self.broker_connected = false;
                            self.live_account = None;
                            self.live_positions.clear(); self.live_positions_by_symbol.clear();
                            self.live_orders.clear(); self.live_orders_by_id.clear();
                            self.pending_alpaca_fetches.clear();
                            self.alpaca_full_bar_sync_enabled = false;
                            self.log.push_back(LogEntry::info("Alpaca disabled — stopped UI-side login/sync/position/order activity. Existing cache data was not deleted."));
                        }
                    }
                    let kr_before = self.kraken_enabled;
                    if ui.checkbox(&mut self.kraken_enabled, "Enable Kraken").on_hover_text("When off: no startup login, private REST/WS account sync, Kraken Spot/Equities/Futures bar sync, or Kraken order buttons. Stored data is left untouched.").changed() {
                        settings_save_after = true;
                        if kr_before && !self.kraken_enabled {
                            self.kraken_connected = false;
                            self.kr_positions.clear(); self.kr_positions_by_symbol.clear(); self.kr_position_asset_tails.clear();
                            self.kraken_balances.clear(); self.kraken_balance_assets_by_display.clear();
                            self.kraken_open_orders.clear();
                            self.kraken_pairs.clear();
                            self.kraken_pairs_normalized.clear();
                            self.kraken_equity_pair_by_base.clear();
                            self.pending_kraken_fetches.clear();
                            self.pending_kraken_futures_fetches.clear();
                            self.kraken_full_bar_sync_enabled = false;
                            self.log.push_back(LogEntry::info("Kraken disabled — stopped UI-side login/sync/position/order activity. Existing cache data was not deleted."));
                        }
                    }
                });
                if self.alpaca_enabled {
                    ui.add_space(4.0);
                    self.render_alpaca_sync_profile_controls(ui, &mut settings_save_after, "settings");
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let connect_label = if self.broker_connected {
                        egui::RichText::new("Alpaca Connected").color(UP)
                    } else {
                        egui::RichText::new("Connect Alpaca")
                    };
                    if ui.add_enabled(self.alpaca_enabled, egui::Button::new(connect_label)).clicked() && !self.broker_connected {
                        if !self.broker_api_key.is_empty() && !self.broker_secret.is_empty() {
                            // Save credentials to system keyring and log the saved credential names only.
                            let mut saved_credentials: Vec<String> = Vec::new();
                            if let Err(e) = keyring::store(keyring::keys::ALPACA_API_KEY, &self.broker_api_key) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store alpaca_api_key failed: {}", e)));
                            } else {
                                saved_credentials.push("alpaca_api_key".into());
                            }
                            if let Err(e) = keyring::store(keyring::keys::ALPACA_SECRET, &self.broker_secret) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store alpaca_secret failed: {}", e)));
                            } else {
                                saved_credentials.push("alpaca_secret".into());
                            }
                            // Extra Alpaca account slots (2–4): store or clear.
                            for (idx, acct) in self.alpaca_extra_accounts.iter().enumerate() {
                                let slot = idx + 2;
                                let (key_name, secret_name) = super::broker_accounts::alpaca_slot_keyring_keys(slot);
                                if acct.api_key.trim().is_empty() || acct.secret.trim().is_empty() {
                                    let _ = keyring::delete(&key_name);
                                    let _ = keyring::delete(&secret_name);
                                    continue;
                                }
                                match (
                                    keyring::store(&key_name, &acct.api_key),
                                    keyring::store(&secret_name, &acct.secret),
                                ) {
                                    (Ok(()), Ok(())) => {
                                        saved_credentials.push(key_name.clone());
                                        saved_credentials.push(secret_name.clone());
                                    }
                                    _ => self.log.push_back(LogEntry::warn(format!(
                                        "Keyring store for Alpaca slot {slot} failed"
                                    ))),
                                }
                            }
                            if !self.finnhub_key.is_empty() {
                                if let Err(e) = keyring::store(keyring::keys::FINNHUB_KEY, &self.finnhub_key) {
                                    self.log.push_back(LogEntry::warn(format!("Keyring store finnhub_key failed: {}", e)));
                                } else {
                                    saved_credentials.push("finnhub_key".into());
                                }
                            }
                            if !self.cryptopanic_key.is_empty() {
                                if let Err(e) = keyring::store(keyring::keys::CRYPTOPANIC_KEY, &self.cryptopanic_key) {
                                    self.log.push_back(LogEntry::warn(format!("Keyring store cryptopanic_key failed: {}", e)));
                                } else {
                                    saved_credentials.push("cryptopanic_key".into());
                                }
                            }
                            if !self.fred_key.is_empty() {
                                if let Err(e) = keyring::store(keyring::keys::FRED_KEY, &self.fred_key) {
                                    self.log.push_back(LogEntry::warn(format!("Keyring store fred_key failed: {}", e)));
                                } else {
                                    saved_credentials.push("fred_key".into());
                                }
                            }
                            if !saved_credentials.is_empty() {
                                self.log.push_back(LogEntry::info(format!(
                                    "Credentials saved to keyring: {}",
                                    saved_credentials.join(", ")
                                )));
                            }
                            self.send_alpaca_connect();
                            settings_save_after = true;
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
                            let mut saved_credentials: Vec<String> = Vec::new();
                            if let Err(e) = keyring::store(keyring::keys::KRAKEN_API_KEY, &self.kraken_api_key) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store kraken_api_key failed: {}", e)));
                            } else {
                                saved_credentials.push("kraken_api_key".into());
                            }
                            if let Err(e) = keyring::store(keyring::keys::KRAKEN_API_SECRET, &self.kraken_api_secret) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store kraken_api_secret failed: {}", e)));
                            } else {
                                saved_credentials.push("kraken_api_secret".into());
                            }
                            if let Err(e) = keyring::store(keyring::keys::KRAKEN_WS_API_KEY, &self.kraken_ws_api_key) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store kraken_ws_api_key failed: {}", e)));
                            } else {
                                saved_credentials.push("kraken_ws_api_key".into());
                            }
                            if let Err(e) = keyring::store(keyring::keys::KRAKEN_WS_API_SECRET, &self.kraken_ws_api_secret) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store kraken_ws_api_secret failed: {}", e)));
                            } else {
                                saved_credentials.push("kraken_ws_api_secret".into());
                            }
                            // Extra Kraken account slots (2–4): store or clear.
                            for (idx, acct) in self.kraken_extra_accounts.iter().enumerate() {
                                let slot = idx + 2;
                                let (key_name, secret_name) = super::broker_accounts::kraken_slot_keyring_keys(slot);
                                if acct.api_key.trim().is_empty() || acct.secret.trim().is_empty() {
                                    let _ = keyring::delete(&key_name);
                                    let _ = keyring::delete(&secret_name);
                                    continue;
                                }
                                match (
                                    keyring::store(&key_name, &acct.api_key),
                                    keyring::store(&secret_name, &acct.secret),
                                ) {
                                    (Ok(()), Ok(())) => {
                                        saved_credentials.push(key_name.clone());
                                        saved_credentials.push(secret_name.clone());
                                    }
                                    _ => self.log.push_back(LogEntry::warn(format!(
                                        "Keyring store for Kraken slot {slot} failed"
                                    ))),
                                }
                            }
                            if !saved_credentials.is_empty() {
                                self.log.push_back(LogEntry::info(format!(
                                    "Credentials saved to keyring: {}",
                                    saved_credentials.join(", ")
                                )));
                            }
                            let _ = self.broker_tx.send(BrokerCmd::KrakenConnect {
                                api_key: self.kraken_api_key.clone(),
                                api_secret: self.kraken_api_secret.clone(),
                                ws_api_key: self.kraken_ws_api_key.clone(),
                                ws_api_secret: self.kraken_ws_api_secret.clone(),
                                extra_accounts: self.kraken_extra_account_specs(),
                                primary_paper: self.kraken_paper,
                            });
                            self.log.push_back(LogEntry::info("Kraken — connecting..."));
                            settings_save_after = true;
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
                            let mut saved_credentials: Vec<&'static str> = Vec::new();
                            if let Err(e) = keyring::store(keyring::keys::MATRIX_ACCESS_TOKEN, &self.matrix_access_token) {
                                self.log.push_back(LogEntry::warn(format!("Keyring store matrix_access_token failed: {}", e)));
                            } else {
                                saved_credentials.push("matrix_access_token");
                            }
                            if !self.matrix_user_id.is_empty() {
                                if let Err(e) = keyring::store(keyring::keys::MATRIX_USER_ID, &self.matrix_user_id) {
                                    self.log.push_back(LogEntry::warn(format!("Keyring store matrix_user_id failed: {}", e)));
                                } else {
                                    saved_credentials.push("matrix_user_id");
                                }
                            }
                            if !saved_credentials.is_empty() {
                                self.log.push_back(LogEntry::info(format!(
                                    "Credentials saved to keyring: {}",
                                    saved_credentials.join(", ")
                                )));
                            }
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
                    ui.checkbox(&mut self.fund_source_alpaca, "Alpaca");
                    ui.checkbox(&mut self.fund_source_kraken, "Kraken");
                });
                // Sync broker_scope from checkbox state
                self.broker_scope = match (self.fund_source_alpaca, self.fund_source_kraken) {
                    (true, false) => EventSource::Alpaca,
                    (false, true) => EventSource::Kraken,
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
                    let alpaca_mode = if self.alpaca_full_bar_sync_enabled {
                        "full Alpaca universe bar sync enabled"
                    } else if self.backfill_alpaca_kraken_equities_enabled {
                        "Kraken-equities assist only; broad Alpaca universe sync off"
                    } else {
                        "account/trading only; broad Alpaca universe sync off"
                    };
                    ui.label(format!("Alpaca: REST API + WebSocket — {} ({})", alpaca_status, alpaca_mode));
                }
                if self.kraken_enabled {
                    let kraken_status = if self.kraken_connected { "Connected" } else { "Disconnected" };
                    let kraken_mode = if self.kraken_full_bar_sync_enabled {
                        "full Kraken selected-universe bar sync enabled"
                    } else {
                        "light sync: open charts/positions/orders/watchlist only"
                    };
                    ui.label(format!("Kraken: REST API + WebSocket — {} ({})", kraken_status, kraken_mode));
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
                    ui.horizontal_wrapped(|ui| {
                        let ws_resp = ui
                            .checkbox(
                                &mut self.kraken_ws_ohlc_enabled,
                                "Stream bars via WebSocket (recommended, on by default)",
                            )
                            .on_hover_text(
                                "Kraken WS v2 OHLC channel pushes bar updates as ticks land — the only way to keep low timeframes healthy across the full Kraken universe (REST's ~55 req/min budget can't refresh 13k pairs × 1Min in 24 minutes). The channel is on Kraken's public WS endpoint (no API key needed) and is strictly better than REST alone for low timeframes. REST keeps doing cold-start historical backfill and high-TF refresh. One TCP connection per interval (8 total) is opened when this is on. Turn off only if you need to suppress the connections for testing or footprint.",
                            );
                        if ws_resp.changed() {
                            settings_save_after = true;
                            if !self.kraken_ws_ohlc_enabled {
                                // Toggle off → mark not-started so a later
                                // toggle-on triggers a fresh spawn. We don't
                                // currently support tearing down running
                                // streamers mid-session; the next session
                                // launches without them.
                                self.kraken_ws_ohlc_started = false;
                                self.kraken_ws_ohlc_streamed_pairs.clear();
                                self.log.push_back(LogEntry::info(
                                    "Kraken WS OHLC disabled — already-running streamers stay live until next launch.",
                                ));
                            } else {
                                self.log.push_back(LogEntry::info(
                                    "Kraken WS OHLC enabled — streamers will spawn once the pair catalog is loaded.",
                                ));
                                // If pairs are already in hand, kick off immediately
                                // instead of waiting for the next KrakenPairs message.
                                self.maybe_start_kraken_ws_ohlc();
                            }
                        }
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
                }
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Full broker bar sync")
                        .color(AXIS_TEXT)
                        .small()
                        .strong(),
                );
                ui.horizontal_wrapped(|ui| {
                    let kraken_resp = ui
                        .add_enabled(
                            self.kraken_enabled,
                            egui::Checkbox::new(
                                &mut self.kraken_full_bar_sync_enabled,
                                "Kraken full bar sync",
                            ),
                        )
                        .on_hover_text(
                            "Explicit opt-in for rotating through selected Kraken Spot/Equities/Futures universes. Off keeps light sync: open charts, positions, open orders, and watchlist only.",
                        );
                    if kraken_resp.changed() {
                        settings_save_after = true;
                        self.pending_kraken_fetches.clear();
                        self.pending_kraken_futures_fetches.clear();
                    }

                    let alpaca_resp = ui
                        .add_enabled(
                            self.alpaca_enabled,
                            egui::Checkbox::new(
                                &mut self.alpaca_full_bar_sync_enabled,
                                "Alpaca full bar sync",
                            ),
                        )
                        .on_hover_text(
                            "Explicit opt-in for broad Alpaca equity-universe historical bar rotation. Off still allows account/trading, light sync, and targeted Kraken-equities fallback assist.",
                        );
                    if alpaca_resp.changed() {
                        settings_save_after = true;
                        self.all_broker_assets_fetched = false;
                        self.pending_alpaca_fetches.clear();
                    }
                });
                ui.label(
                    egui::RichText::new("Default light sync still refreshes manually opened symbols, broker positions/open orders, and watchlist symbols.")
                        .color(AXIS_TEXT)
                        .small(),
                );
                if self.backfill_alpaca_kraken_equities_enabled && !self.alpaca_full_bar_sync_enabled {
                    ui.label(
                        egui::RichText::new("Kraken assist uses targeted Alpaca fetches only")
                            .color(AXIS_TEXT)
                            .small(),
                    );
                }
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Backfill providers")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    let mut backfill_changed = false;
                    ui.horizontal_wrapped(|ui| {
                        backfill_changed |= ui
                            .checkbox(
                                &mut self.backfill_alpaca_kraken_equities_enabled,
                                "Alpaca for all Kraken equities",
                            )
                            .on_hover_text(
                                "When enabled, every Kraken equities/xStocks candidate may use Alpaca as provenance-tagged gap-fill. This is assist-only fallback, not broad Alpaca universe sync.",
                            )
                            .changed();
                        backfill_changed |= ui
                            .checkbox(
                                &mut self.backfill_yahoo_chart_enabled,
                                "Yahoo Chart fallback",
                            )
                            .on_hover_text(
                                "Best-effort unkeyed equity/ETF chart fallback. Use only as lower-trust gap fill with provenance, never as authoritative broker data.",
                            )
                            .changed();
                    });
                    if backfill_changed {
                        settings_save_after = true;
                        self.pending_kraken_fetches.clear();
                    }
                    ui.label(
                        egui::RichText::new(
                            "Backfill providers supplement native broker bars. They must preserve source provenance; enabling Alpaca here means Kraken-equity assist, not a full Alpaca universe pull.",
                        )
                        .color(AXIS_TEXT)
                        .small(),
                    );
                ui.label(
                    egui::RichText::new(
                        "These category and quote filters control automated broker public scraping. Kraken uses them now; future crypto brokers should call the same quote filter instead of adding broker-specific assumptions.",
                    )
                    .color(AXIS_TEXT)
                    .small(),
                );
                ui.label("Finnhub: News, Analyst, Insider Sentiment, Short Interest");
                ui.label("SEC EDGAR: Filing scraper + Form 4 insider trades");
                ui.add_space(6.0);
                ui.separator();
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
        // Apply dynamic account add/remove now that the grid closure has released
        // its borrow of `self`. Remove before add so a same-frame combination is
        // unambiguous; each mutates the slot Vec + keyring and flags a save.
        if let Some(idx) = alpaca_remove_account {
            self.remove_alpaca_account(idx);
            settings_save_after = true;
        }
        if alpaca_add_account {
            self.add_alpaca_account();
            settings_save_after = true;
        }
        if let Some(idx) = kraken_remove_account {
            self.remove_kraken_account(idx);
            settings_save_after = true;
        }
        if kraken_add_account {
            self.add_kraken_account();
            settings_save_after = true;
        }
        self.show_settings = show_settings;
        settings_save_after
    }
}
