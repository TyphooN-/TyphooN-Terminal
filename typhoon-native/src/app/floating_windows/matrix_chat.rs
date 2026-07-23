use super::*;

impl TyphooNApp {
    pub(super) fn render_matrix_chat_window(&mut self, ctx: &egui::Context) {
        // Matrix Chat (public room viewer)
        if self.show_matrix_chat {
            // Try to load Matrix token from keyring on first open
            if self.matrix_access_token.is_empty() && !self.matrix_room.is_empty() {
                // Check keyring for stored token
                if let Ok(Some(token)) = typhoon_engine::core::keyring::load(
                    typhoon_engine::core::keyring::keys::MATRIX_ACCESS_TOKEN,
                ) {
                    self.matrix_access_token = token;
                    if let Ok(Some(uid)) = typhoon_engine::core::keyring::load(
                        typhoon_engine::core::keyring::keys::MATRIX_USER_ID,
                    ) {
                        self.matrix_user_id = uid;
                    }
                    self.log.push_back(LogEntry::info(format!(
                        "Matrix: restored session as {}",
                        self.matrix_user_id
                    )));
                    // Join room + fetch
                    let _ = self.broker_tx.send(BrokerCmd::MatrixJoinRoom {
                        room_id: self.matrix_room.clone(),
                        access_token: self.matrix_access_token.clone(),
                    });
                    let _ = self.broker_tx.send(BrokerCmd::MatrixFetchMessages {
                        room_id: self.matrix_room.clone(),
                        access_token: self.matrix_access_token.clone(),
                    });
                } else {
                    self.matrix_access_token = "none".to_string(); // mark as checked
                    self.log.push_back(LogEntry::info(
                        "Matrix: no access token — read-only mode. Set token in Settings.",
                    ));
                    // Fetch without auth (read-only for world-readable rooms)
                    let _ = self.broker_tx.send(BrokerCmd::MatrixFetchMessages {
                        room_id: self.matrix_room.clone(),
                        access_token: String::new(),
                    });
                }
            }
            // Auto-refresh every 10 seconds
            if self.matrix_last_fetch.elapsed() > std::time::Duration::from_secs(10)
                && !self.matrix_access_token.is_empty()
                && self.matrix_access_token != "pending"
            {
                let _ = self.broker_tx.send(BrokerCmd::MatrixFetchMessages {
                    room_id: self.matrix_room.clone(),
                    access_token: self.matrix_access_token.clone(),
                });
                self.matrix_last_fetch = std::time::Instant::now();
            }
            egui::Window::new("Community Chat")
                .open(&mut self.show_matrix_chat)
                .resizable(true)
                .default_size([500.0, 450.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("TyphooN Terminal Community").strong());
                        let status = if self.matrix_access_token == "pending" {
                            ("Joining...", egui::Color32::YELLOW)
                        } else if !self.matrix_access_token.is_empty() {
                            ("Connected", egui::Color32::from_rgb(80, 220, 120))
                        } else {
                            ("Disconnected", egui::Color32::from_rgb(255, 80, 80))
                        };
                        ui.label(egui::RichText::new(status.0).small().color(status.1));
                        if !self.matrix_user_id.is_empty() {
                            ui.label(
                                egui::RichText::new(&self.matrix_user_id)
                                    .small()
                                    .color(AXIS_TEXT),
                            );
                        }
                    });
                    ui.separator();
                    // Messages — fill available height, leaving room for the input row below, so
                    // the message list grows with the window instead of being pinned to 350px.
                    let msg_h = (ui.available_height() - 48.0).max(80.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .stick_to_bottom(true)
                        .max_height(msg_h)
                        .show(ui, |ui| {
                            if self.matrix_messages.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(40.0);
                                    ui.label(
                                        egui::RichText::new("Connecting to community chat...")
                                            .color(AXIS_TEXT),
                                    );
                                });
                            }
                            for (sender, ts, body) in &self.matrix_messages {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        egui::RichText::new(ts)
                                            .color(AXIS_TEXT)
                                            .small()
                                            .monospace(),
                                    );
                                    let sender_hash =
                                        sender.bytes().fold(0u8, |a, b| a.wrapping_add(b)) as usize;
                                    let sender_col = WL_COLORS[sender_hash % WL_COLORS.len()];
                                    ui.label(
                                        egui::RichText::new(sender)
                                            .color(sender_col)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(egui::RichText::new(body).small());
                                });
                            }
                        });
                    ui.separator();
                    // Send message input
                    if self.matrix_access_token.is_empty()
                        || self.matrix_access_token == "pending"
                        || self.matrix_access_token == "none"
                    {
                        ui.label(
                            egui::RichText::new("Read-only — set Matrix access token in Settings")
                                .small()
                                .color(AXIS_TEXT),
                        );
                    } else {
                        ui.horizontal(|ui| {
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.matrix_input)
                                    .desired_width(ui.available_width() - 60.0)
                                    .hint_text("Type a message..."),
                            );
                            let send = ui.button("Send").clicked()
                                || (resp.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                            if send && !self.matrix_input.trim().is_empty() {
                                let body = self.matrix_input.trim().to_string();
                                self.matrix_input.clear();
                                let _ = self.broker_tx.send(BrokerCmd::MatrixSendMessage {
                                    room_id: self.matrix_room.clone(),
                                    access_token: self.matrix_access_token.clone(),
                                    body,
                                });
                            }
                        });
                    }
                });
        }
    }
}
