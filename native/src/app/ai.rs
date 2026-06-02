use super::*;

impl TyphooNApp {
    pub(super) fn render_ai_chat_window(&mut self, ctx: &egui::Context) {
        if self.show_ai_chat {
            let mut save_askai_transcript: Option<(String, String)> = None;
            let mut matrix_askai_transcript: Option<(String, String)> = None;
            egui::Window::new("AI Assistant")
                .open(&mut self.show_ai_chat)
                .resizable(true)
                .default_size([560.0, 520.0])
                .min_width(460.0)
                .min_height(300.0)
                .constrain(true)
                .show(ctx, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Provider:");
                        ui.radio_value(&mut self.ai_provider, 0, "Claude");
                        ui.radio_value(&mut self.ai_provider, 1, "GPT");
                        ui.radio_value(&mut self.ai_provider, 2, "Gemini");
                        ui.radio_value(&mut self.ai_provider, 3, "Grok");
                        ui.radio_value(&mut self.ai_provider, 4, "Mistral");
                        ui.radio_value(&mut self.ai_provider, 5, "Perplexity");
                        ui.radio_value(&mut self.ai_provider, 6, "Local");
                        let has_turns = !self.ai_chat_history.is_empty();
                        ui.add_enabled_ui(has_turns, |ui| {
                            let provider_label = match self.ai_provider {
                                0 => "Claude",
                                1 => "GPT",
                                2 => "Gemini",
                                3 => "Grok",
                                4 => "Mistral",
                                5 => "Perplexity",
                                _ => "Local",
                            };
                            let slug = provider_label.to_ascii_lowercase();
                            if ui
                                .button("\u{1F4BE} Save")
                                .on_hover_text("Export this ASKAI session to a markdown file")
                                .clicked()
                            {
                                let transcript = Self::format_ai_transcript(
                                    &self.ai_chat_history,
                                    &format!("ASKAI / {provider_label}"),
                                    provider_label,
                                    Some(self.ai_chat_session_id.as_str()),
                                );
                                save_askai_transcript = Some((format!("askai_{slug}"), transcript));
                            }
                            if ui
                                .button("\u{1F4E8} Matrix")
                                .on_hover_text("Post this ASKAI session to the Community Chat room")
                                .clicked()
                            {
                                let transcript = Self::format_ai_transcript(
                                    &self.ai_chat_history,
                                    &format!("ASKAI / {provider_label}"),
                                    provider_label,
                                    Some(self.ai_chat_session_id.as_str()),
                                );
                                matrix_askai_transcript =
                                    Some((format!("askai_{slug}"), transcript));
                            }
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Model:");
                        let (cb_id, options): (&str, &[(&str, &str)]) = match self.ai_provider {
                            0 => (
                                "ai_model_claude",
                                &[
                                    ("claude-opus-4-5", "opus 4.5 (max effort)"),
                                    ("claude-sonnet-4-5", "sonnet 4.5 (balanced)"),
                                    ("claude-haiku-4-5", "haiku 4.5 (fast)"),
                                ],
                            ),
                            1 => (
                                "ai_model_openai",
                                &[
                                    ("gpt-4o", "gpt-4o"),
                                    ("gpt-4o-mini", "gpt-4o-mini"),
                                    ("o1-preview", "o1-preview"),
                                ],
                            ),
                            2 => ("ai_model_gemini", Self::gemini_cli_model_options()),
                            3 => (
                                "ai_model_grok",
                                &[("grok-3", "grok-3"), ("grok-3-mini", "grok-3-mini")],
                            ),
                            4 => (
                                "ai_model_mistral",
                                &[
                                    ("mistral-large-latest", "mistral-large"),
                                    ("mistral-small-latest", "mistral-small"),
                                ],
                            ),
                            5 => (
                                "ai_model_perplexity",
                                &[("sonar-pro", "sonar-pro"), ("sonar", "sonar")],
                            ),
                            _ => (
                                "ai_model_local",
                                &[("llama3.2", "llama3.2"), ("qwen2.5:32b", "qwen2.5:32b")],
                            ),
                        };
                        if self.ai_model.is_empty()
                            || !options.iter().any(|(v, _)| *v == self.ai_model)
                        {
                            self.ai_model = options[0].0.to_string();
                        }
                        egui::ComboBox::from_id_salt(cb_id)
                            .selected_text(self.ai_model.as_str())
                            .show_ui(ui, |ui| {
                                for (value, label) in options {
                                    ui.selectable_value(
                                        &mut self.ai_model,
                                        value.to_string(),
                                        *label,
                                    );
                                }
                            });
                        if self.ai_chat_packet.is_some() {
                            ui.label(egui::RichText::new("[packet loaded]").small().color(UP));
                        }
                    });
                    ui.separator();
                    let scroll_h = (ui.available_height() - 60.0).max(120.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(scroll_h)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            if self.ai_chat_history.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(40.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "Ask the hosted AI provider — pick model above.",
                                        )
                                        .color(AXIS_TEXT),
                                    );
                                });
                            }
                            for (is_user, msg) in &self.ai_chat_history {
                                let (align, color, prefix) = if *is_user {
                                    (
                                        egui::Align::RIGHT,
                                        egui::Color32::from_rgb(80, 140, 255),
                                        "You",
                                    )
                                } else {
                                    (
                                        egui::Align::LEFT,
                                        egui::Color32::from_rgb(180, 180, 200),
                                        "AI",
                                    )
                                };
                                ui.with_layout(egui::Layout::top_down(align), |ui| {
                                    ui.label(
                                        egui::RichText::new(prefix).strong().small().color(color),
                                    );
                                    ui.label(egui::RichText::new(msg).small());
                                });
                                ui.add_space(4.0);
                            }
                        });
                    ui.separator();
                    ui.horizontal(|ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.ai_chat_input)
                                .desired_width(ui.available_width() - 60.0)
                                .hint_text("Ask anything..."),
                        );
                        let send = ui.button("Send").clicked()
                            || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                        if send && !self.ai_chat_input.is_empty() {
                            let msg = self.ai_chat_input.clone();
                            self.ai_chat_history.push((true, msg.clone()));
                            let (provider, key) = match self.ai_provider {
                                0 => ("claude", self.anthropic_key.clone()),
                                1 => ("openai", self.openai_key.clone()),
                                2 => ("gemini", self.gemini_key.clone()),
                                3 => ("grok", self.xai_key.clone()),
                                4 => ("mistral", self.mistral_key.clone()),
                                5 => ("perplexity", self.perplexity_key.clone()),
                                6 => ("local", "http://localhost:11434".to_string()),
                                _ => ("openai", self.openai_key.clone()),
                            };
                            if key.is_empty() && self.ai_provider != 6 {
                                self.ai_chat_history
                                    .push((false, "Set API key in Settings first.".into()));
                            } else {
                                let _ = self.broker_tx.send(BrokerCmd::AiChat {
                                    provider: provider.into(),
                                    api_key: key,
                                    message: msg,
                                    history: self.ai_chat_history.clone(),
                                    system: self.ai_chat_packet.clone(),
                                    model: Some(self.ai_model.clone()),
                                });
                            }
                            self.ai_chat_input.clear();
                        }
                    });
                });
            if let Some((slug, t)) = save_askai_transcript {
                self.save_ai_session_to_file(&slug, &t);
            }
            if let Some((slug, t)) = matrix_askai_transcript {
                self.send_ai_session_to_matrix(&slug, &t);
            }
        }
    }

    pub(super) fn render_claude_code_window(&mut self, ctx: &egui::Context) {
        // Drain responses from background thread
        if let Some(ref rx) = self.claude_code_rx {
            if let Ok(response) = rx.try_recv() {
                self.maybe_queue_ingest_from_ai_response("claude", &response);
                self.claude_code_history.push((false, response));
                self.claude_code_rx = None;
                // Persist the turn. Claude's CLI UUID doubles as our kv key so
                // RESUMECLAUDE can restore --resume continuity.
                if let Some(ref sid) = self.claude_code_session_id {
                    let sid = sid.clone();
                    let model = self.claude_model.clone();
                    let history = self.claude_code_history.clone();
                    self.persist_ai_turn("claude", &sid, Some(&sid), &history, &model);
                }
            }
        }
        if self.show_claude_code {
            let mut save_claude_transcript: Option<String> = None;
            let mut matrix_claude_transcript: Option<String> = None;
            egui::Window::new("Claude Code")
                .open(&mut self.show_claude_code)
                .resizable(true).default_size([620.0, 520.0])
                .min_width(420.0).min_height(280.0)
                .constrain(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Claude Code CLI — local subscription").small().color(AXIS_TEXT));
                        ui.separator();
                        ui.label("Model:");
                        egui::ComboBox::from_id_salt("claude_model_picker")
                            .selected_text(self.claude_model.as_str())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.claude_model, "opus".to_string(), "opus (max effort)");
                                ui.selectable_value(&mut self.claude_model, "sonnet".to_string(), "sonnet (balanced)");
                                ui.selectable_value(&mut self.claude_model, "haiku".to_string(), "haiku (fast)");
                            });
                        ui.label("Effort:");
                        egui::ComboBox::from_id_salt("claude_effort_picker")
                            .selected_text(Self::claude_effort_label(&self.claude_effort))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.claude_effort, "ultrathink".to_string(),   "max (ultrathink)");
                                ui.selectable_value(&mut self.claude_effort, "think harder".to_string(), "high (think harder)");
                                ui.selectable_value(&mut self.claude_effort, "think hard".to_string(),   "medium (think hard)");
                                ui.selectable_value(&mut self.claude_effort, "think".to_string(),        "low (think)");
                                ui.selectable_value(&mut self.claude_effort, "".to_string(),             "off");
                            });
                        if self.claude_code_packet.is_some() {
                            ui.label(egui::RichText::new("[packet loaded]").small().color(UP));
                        }
                        if self.claude_code_session_id.is_some() {
                            ui.label(egui::RichText::new("[session continued]").small().color(AXIS_TEXT));
                        }
                        let has_turns = !self.claude_code_history.is_empty();
                        ui.add_enabled_ui(has_turns, |ui| {
                            if ui.button("\u{1F4BE} Save")
                                .on_hover_text("Export this Claude Code session to a markdown file")
                                .clicked()
                            {
                                save_claude_transcript = Some(Self::format_ai_transcript(
                                    &self.claude_code_history, "Claude Code", "Claude",
                                    self.claude_code_session_id.as_deref(),
                                ));
                            }
                            if ui.button("\u{1F4E8} Matrix")
                                .on_hover_text("Post this Claude Code session to the Community Chat room")
                                .clicked()
                            {
                                matrix_claude_transcript = Some(Self::format_ai_transcript(
                                    &self.claude_code_history, "Claude Code", "Claude",
                                    self.claude_code_session_id.as_deref(),
                                ));
                            }
                        });
                    });
                    ui.separator();
                    // Reserve ~60px for input row + separator so scroll fills the window.
                    let scroll_h = (ui.available_height() - 60.0).max(120.0);
                    egui::ScrollArea::vertical().auto_shrink(false).max_height(scroll_h).stick_to_bottom(true).show(ui, |ui| {
                        if self.claude_code_history.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(40.0);
                                ui.label(egui::RichText::new("Ask Claude anything — uses your local claude CLI").color(AXIS_TEXT));
                            });
                        }
                        for (is_user, msg) in &self.claude_code_history {
                            let (align, color, prefix) = if *is_user {
                                (egui::Align::RIGHT, egui::Color32::from_rgb(80, 140, 255), "You")
                            } else {
                                (egui::Align::LEFT, egui::Color32::from_rgb(220, 180, 100), "Claude")
                            };
                            ui.with_layout(egui::Layout::top_down(align), |ui| {
                                ui.label(egui::RichText::new(prefix).strong().small().color(color));
                                ui.label(egui::RichText::new(msg).small());
                            });
                            ui.add_space(4.0);
                        }
                        if self.claude_code_rx.is_some() {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(egui::RichText::new("Thinking...").small().color(AXIS_TEXT));
                            });
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        let resp = ui.add(egui::TextEdit::singleline(&mut self.claude_code_input)
                            .desired_width(ui.available_width() - 60.0)
                            .hint_text("Ask Claude..."));
                        let send = ui.button("Send").clicked()
                            || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                        if send && !self.claude_code_input.trim().is_empty() && self.claude_code_rx.is_none() {
                            let msg = self.claude_code_input.trim().to_string();
                            self.claude_code_input.clear();
                            self.claude_code_history.push((true, msg.clone()));

                            let lower = msg.to_lowercase();
                            let builtin_reply: Option<String> = match lower.as_str() {
                                "/clear" => {
                                    self.claude_code_history.clear();
                                    self.claude_code_history.push((false, "(chat history cleared)".to_string()));
                                    self.claude_code_session_id = None;
                                    None
                                }
                                "/help" => Some(
                                    "Local chat help:\n\
                                     • Type any prompt and press Enter to ask Claude.\n\
                                     • /clear — clear the chat history and session\n\
                                     • /status — show local chat status\n\
                                     • /help — this message\n\
                                     \n\
                                     The research packet and conversation history are injected \
                                     into every prompt so Claude can answer follow-ups without \
                                     losing context.".to_string()
                                ),
                                "/status" => {
                                    let count = self.claude_code_history.iter().filter(|(u, _)| *u).count();
                                    let has_pkt = if self.claude_code_packet.is_some() { "yes" } else { "no" };
                                    let sess = self.claude_code_session_id.as_deref().unwrap_or("(new)");
                                    Some(format!(
                                        "Local chat status:\n\
                                         • Backend: `claude --print` subprocess\n\
                                         • Model: {}\n\
                                         • Effort: {}\n\
                                         • Research packet loaded: {has_pkt}\n\
                                         • Messages this session: {count}\n\
                                         • Session id: {sess}\n\
                                         • Allowed tools: WebSearch, WebFetch, Read, Grep, Glob, Bash",
                                         self.claude_model,
                                         Self::claude_effort_label(&self.claude_effort)
                                    ))
                                }
                                _ => {
                                    const INTERACTIVE_ONLY: &[&str] = &[
                                        "/model", "/cost", "/config", "/login", "/logout",
                                        "/permissions", "/theme", "/mcp", "/ide", "/exit",
                                        "/compact", "/resume", "/bug", "/release-notes",
                                    ];
                                    if INTERACTIVE_ONLY.iter().any(|c| lower == *c) {
                                        Some(format!(
                                            "`{msg}` is a Claude Code interactive command and \
                                             cannot be invoked through `claude --print`. Run it in \
                                             a real terminal instead."
                                        ))
                                    } else {
                                        None
                                    }
                                }
                            };

                            if let Some(reply) = builtin_reply {
                                self.claude_code_history.push((false, reply));
                                return;
                            }
                            if lower == "/clear" {
                                return;
                            }

                            // Build full prompt: research packet + transcript + new msg.
                            let full_prompt = Self::build_claude_prompt(
                                self.claude_code_packet.as_deref(),
                                &self.claude_code_history,
                                &msg,
                                &self.claude_effort,
                            );
                            let model = self.claude_model.clone();
                            // Reuse per-window session UUID so Claude CLI resumes the same thread.
                            let session_id = self
                                .claude_code_session_id
                                .get_or_insert_with(Self::new_uuid)
                                .clone();
                            let is_first = self.claude_code_history.iter().filter(|(u, _)| *u).count() <= 1;

                            let (tx, rx) = std::sync::mpsc::channel();
                            self.claude_code_rx = Some(rx);
                            Self::spawn_claude_print(model, session_id, is_first, full_prompt, tx);
                        }
                    });
                });
            if let Some(t) = save_claude_transcript {
                self.save_ai_session_to_file("claude_code", &t);
            }
            if let Some(t) = matrix_claude_transcript {
                self.send_ai_session_to_matrix("claude_code", &t);
            }
        }
    }

    pub(super) fn render_gemini_cli_window(&mut self, ctx: &egui::Context) {
        if let Some(ref rx) = self.gemini_cli_rx {
            if let Ok(response) = rx.try_recv() {
                self.maybe_queue_ingest_from_ai_response("gemini", &response);
                self.gemini_cli_history.push((false, response));
                self.gemini_cli_rx = None;
                let sid = Self::ensure_session_id(&mut self.gemini_cli_session_id);
                let model = self.gemini_model.clone();
                let history = self.gemini_cli_history.clone();
                self.persist_ai_turn("gemini", &sid, None, &history, &model);
            }
        }
        if self.show_gemini_cli {
            let mut save_gemini_transcript: Option<String> = None;
            let mut matrix_gemini_transcript: Option<String> = None;
            egui::Window::new("Gemini CLI")
                .open(&mut self.show_gemini_cli)
                .resizable(true)
                .default_size([620.0, 520.0])
                .min_width(420.0)
                .min_height(280.0)
                .constrain(true)
                .show(ctx, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new("Gemini CLI — local binary")
                                .small()
                                .color(AXIS_TEXT),
                        );
                        ui.separator();
                        ui.label("Model:");
                        egui::ComboBox::from_id_salt("gemini_model_picker")
                            .selected_text(self.gemini_model.as_str())
                            .show_ui(ui, |ui| {
                                for (value, label) in Self::gemini_cli_model_options() {
                                    ui.selectable_value(
                                        &mut self.gemini_model,
                                        value.to_string(),
                                        *label,
                                    );
                                }
                            });
                        ui.add(
                            egui::TextEdit::singleline(&mut self.gemini_model)
                                .desired_width(180.0)
                                .hint_text("any gemini CLI model id"),
                        )
                        .on_hover_text(
                            "Type any model your installed gemini CLI/account can use. TyphooN passes it through to `gemini --model`; unsupported/limited models report the CLI error.",
                        );
                        ui.label(
                            egui::RichText::new("usage shown after each reply; remaining quota unavailable")
                                .small()
                                .color(AXIS_TEXT),
                        );
                        if self.gemini_cli_packet.is_some() {
                            ui.label(egui::RichText::new("[packet loaded]").small().color(UP));
                        }
                        let has_turns = !self.gemini_cli_history.is_empty();
                        ui.add_enabled_ui(has_turns, |ui| {
                            if ui
                                .button("\u{1F4BE} Save")
                                .on_hover_text("Export this Gemini session to a markdown file")
                                .clicked()
                            {
                                save_gemini_transcript = Some(Self::format_ai_transcript(
                                    &self.gemini_cli_history,
                                    "Gemini CLI",
                                    "Gemini",
                                    Some(self.gemini_cli_session_id.as_str()),
                                ));
                            }
                            if ui
                                .button("\u{1F4E8} Matrix")
                                .on_hover_text(
                                    "Post this Gemini session to the Community Chat room",
                                )
                                .clicked()
                            {
                                matrix_gemini_transcript = Some(Self::format_ai_transcript(
                                    &self.gemini_cli_history,
                                    "Gemini CLI",
                                    "Gemini",
                                    Some(self.gemini_cli_session_id.as_str()),
                                ));
                            }
                        });
                    });
                    ui.separator();
                    let scroll_h = (ui.available_height() - 60.0).max(120.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(scroll_h)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            if self.gemini_cli_history.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(40.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "Ask Gemini anything — uses your local gemini CLI",
                                        )
                                        .color(AXIS_TEXT),
                                    );
                                });
                            }
                            for (is_user, msg) in &self.gemini_cli_history {
                                let (align, color, prefix) = if *is_user {
                                    (
                                        egui::Align::RIGHT,
                                        egui::Color32::from_rgb(80, 140, 255),
                                        "You",
                                    )
                                } else {
                                    (
                                        egui::Align::LEFT,
                                        egui::Color32::from_rgb(100, 200, 220),
                                        "Gemini",
                                    )
                                };
                                ui.with_layout(egui::Layout::top_down(align), |ui| {
                                    ui.label(
                                        egui::RichText::new(prefix).strong().small().color(color),
                                    );
                                    ui.label(egui::RichText::new(msg).small());
                                });
                                ui.add_space(4.0);
                            }
                            if self.gemini_cli_rx.is_some() {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(
                                        egui::RichText::new("Thinking...").small().color(AXIS_TEXT),
                                    );
                                });
                            }
                        });
                    ui.separator();
                    ui.horizontal(|ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.gemini_cli_input)
                                .desired_width(ui.available_width() - 60.0)
                                .hint_text("Ask Gemini..."),
                        );
                        let send = ui.button("Send").clicked()
                            || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                        if send
                            && !self.gemini_cli_input.trim().is_empty()
                            && self.gemini_cli_rx.is_none()
                        {
                            let msg = self.gemini_cli_input.trim().to_string();
                            self.gemini_cli_input.clear();
                            self.gemini_cli_history.push((true, msg.clone()));
                            let full_prompt = Self::build_claude_prompt(
                                self.gemini_cli_packet.as_deref(),
                                &self.gemini_cli_history,
                                &msg,
                                "",
                            );
                            let model = self.gemini_model.clone();
                            let (tx, rx) = std::sync::mpsc::channel();
                            self.gemini_cli_rx = Some(rx);
                            Self::spawn_gemini_prompt(model, full_prompt, tx);
                        }
                    });
                });
            if let Some(t) = save_gemini_transcript {
                self.save_ai_session_to_file("gemini_cli", &t);
            }
            if let Some(t) = matrix_gemini_transcript {
                self.send_ai_session_to_matrix("gemini_cli", &t);
            }
        }
    }

    pub(super) fn render_codex_cli_window(&mut self, ctx: &egui::Context) {
        if let Some(ref rx) = self.codex_cli_rx {
            if let Ok(response) = rx.try_recv() {
                self.maybe_queue_ingest_from_ai_response("codex", &response);
                self.codex_cli_history.push((false, response));
                self.codex_cli_rx = None;
                let sid = Self::ensure_session_id(&mut self.codex_cli_session_id);
                let model = self.codex_model.clone();
                let history = self.codex_cli_history.clone();
                self.persist_ai_turn("codex", &sid, None, &history, &model);
            }
        }
        if self.show_codex_cli {
            let mut save_codex_transcript: Option<String> = None;
            let mut matrix_codex_transcript: Option<String> = None;
            let mut codex_save_after = false;
            egui::Window::new("Codex CLI")
                .open(&mut self.show_codex_cli)
                .resizable(true)
                .default_size([620.0, 520.0])
                .min_width(420.0)
                .min_height(280.0)
                .constrain(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Codex CLI — local binary")
                                .small()
                                .color(AXIS_TEXT),
                        );
                        ui.separator();
                        ui.label("Model:");
                        let prev_model = self.codex_model.clone();
                        egui::ComboBox::from_id_salt("codex_model_picker")
                            .selected_text(self.codex_model.as_str())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.codex_model,
                                    "gpt-5-codex".to_string(),
                                    "gpt-5-codex",
                                );
                                ui.selectable_value(
                                    &mut self.codex_model,
                                    "gpt-5".to_string(),
                                    "gpt-5",
                                );
                                ui.selectable_value(
                                    &mut self.codex_model,
                                    "o4-mini".to_string(),
                                    "o4-mini",
                                );
                            });
                        if self.codex_model != prev_model {
                            codex_save_after = true;
                        }
                        ui.label("Reasoning:");
                        let prev_effort = self.codex_reasoning_effort.clone();
                        egui::ComboBox::from_id_salt("codex_reasoning_picker")
                            .selected_text(Self::codex_reasoning_effort_label(
                                &self.codex_reasoning_effort,
                            ))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.codex_reasoning_effort,
                                    "default".to_string(),
                                    "model default",
                                );
                                ui.selectable_value(
                                    &mut self.codex_reasoning_effort,
                                    "minimal".to_string(),
                                    "minimal",
                                );
                                ui.selectable_value(
                                    &mut self.codex_reasoning_effort,
                                    "low".to_string(),
                                    "low",
                                );
                                ui.selectable_value(
                                    &mut self.codex_reasoning_effort,
                                    "medium".to_string(),
                                    "medium",
                                );
                                ui.selectable_value(
                                    &mut self.codex_reasoning_effort,
                                    "high".to_string(),
                                    "high",
                                );
                                ui.selectable_value(
                                    &mut self.codex_reasoning_effort,
                                    "xhigh".to_string(),
                                    "max (xhigh)",
                                );
                            });
                        if self.codex_reasoning_effort != prev_effort {
                            self.codex_reasoning_effort = Self::normalize_codex_reasoning_effort(
                                &self.codex_reasoning_effort,
                            )
                            .to_string();
                            codex_save_after = true;
                        }
                        if self.codex_cli_packet.is_some() {
                            ui.label(egui::RichText::new("[packet loaded]").small().color(UP));
                        }
                        let has_turns = !self.codex_cli_history.is_empty();
                        ui.add_enabled_ui(has_turns, |ui| {
                            if ui
                                .button("\u{1F4BE} Save")
                                .on_hover_text("Export this Codex session to a markdown file")
                                .clicked()
                            {
                                save_codex_transcript = Some(Self::format_ai_transcript(
                                    &self.codex_cli_history,
                                    "Codex CLI",
                                    "Codex",
                                    Some(self.codex_cli_session_id.as_str()),
                                ));
                            }
                            if ui
                                .button("\u{1F4E8} Matrix")
                                .on_hover_text("Post this Codex session to the Community Chat room")
                                .clicked()
                            {
                                matrix_codex_transcript = Some(Self::format_ai_transcript(
                                    &self.codex_cli_history,
                                    "Codex CLI",
                                    "Codex",
                                    Some(self.codex_cli_session_id.as_str()),
                                ));
                            }
                        });
                    });
                    ui.separator();
                    let scroll_h = (ui.available_height() - 60.0).max(120.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(scroll_h)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            if self.codex_cli_history.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(40.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "Ask Codex anything — uses your local codex CLI",
                                        )
                                        .color(AXIS_TEXT),
                                    );
                                });
                            }
                            for (is_user, msg) in &self.codex_cli_history {
                                let (align, color, prefix) = if *is_user {
                                    (
                                        egui::Align::RIGHT,
                                        egui::Color32::from_rgb(80, 140, 255),
                                        "You",
                                    )
                                } else {
                                    (
                                        egui::Align::LEFT,
                                        egui::Color32::from_rgb(220, 180, 100),
                                        "Codex",
                                    )
                                };
                                ui.with_layout(egui::Layout::top_down(align), |ui| {
                                    ui.label(
                                        egui::RichText::new(prefix).strong().small().color(color),
                                    );
                                    ui.label(egui::RichText::new(msg).small());
                                });
                                ui.add_space(4.0);
                            }
                            if self.codex_cli_rx.is_some() {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(
                                        egui::RichText::new("Thinking...").small().color(AXIS_TEXT),
                                    );
                                });
                            }
                        });
                    ui.separator();
                    ui.horizontal(|ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.codex_cli_input)
                                .desired_width(ui.available_width() - 60.0)
                                .hint_text("Ask Codex..."),
                        );
                        let send = ui.button("Send").clicked()
                            || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                        if send
                            && !self.codex_cli_input.trim().is_empty()
                            && self.codex_cli_rx.is_none()
                        {
                            let msg = self.codex_cli_input.trim().to_string();
                            self.codex_cli_input.clear();
                            self.codex_cli_history.push((true, msg.clone()));
                            let full_prompt = Self::build_claude_prompt(
                                self.codex_cli_packet.as_deref(),
                                &self.codex_cli_history,
                                &msg,
                                "",
                            );
                            let model = self.codex_model.clone();
                            let reasoning_effort = self.codex_reasoning_effort.clone();
                            let (tx, rx) = std::sync::mpsc::channel();
                            self.codex_cli_rx = Some(rx);
                            Self::spawn_codex_exec(model, reasoning_effort, full_prompt, tx);
                        }
                    });
                });
            if let Some(t) = save_codex_transcript {
                self.save_ai_session_to_file("codex_cli", &t);
            }
            if let Some(t) = matrix_codex_transcript {
                self.send_ai_session_to_matrix("codex_cli", &t);
            }
            if codex_save_after {
                self.save_session();
            }
        }
    }

    pub(super) fn render_hermes_cli_window(&mut self, ctx: &egui::Context) {
        if let Some(ref rx) = self.hermes_cli_rx {
            if let Ok(response) = rx.try_recv() {
                self.maybe_queue_ingest_from_ai_response("hermes", &response);
                self.hermes_cli_history.push((false, response));
                self.hermes_cli_rx = None;
                let sid = Self::ensure_session_id(&mut self.hermes_cli_session_id);
                let model = if self.hermes_model.trim().is_empty() {
                    "configured-default".to_string()
                } else {
                    self.hermes_model.clone()
                };
                let history = self.hermes_cli_history.clone();
                self.persist_ai_turn("hermes", &sid, None, &history, &model);
            }
        }
        if self.show_hermes_cli {
            let mut save_hermes_transcript: Option<String> = None;
            let mut matrix_hermes_transcript: Option<String> = None;
            let mut hermes_save_after = false;
            egui::Window::new("Hermes Agent CLI")
                .open(&mut self.show_hermes_cli)
                .resizable(true)
                .default_size([620.0, 520.0])
                .min_width(420.0)
                .min_height(280.0)
                .constrain(true)
                .show(ctx, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new("Hermes Agent CLI — local hermes binary")
                                .small()
                                .color(AXIS_TEXT),
                        );
                        ui.separator();
                        let prev_model = self.hermes_model.clone();
                        ui.label("Model:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hermes_model)
                                .desired_width(170.0)
                                .hint_text("configured default"),
                        );
                        let prev_provider = self.hermes_provider.clone();
                        ui.label("Provider:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hermes_provider)
                                .desired_width(120.0)
                                .hint_text("default"),
                        );
                        if self.hermes_model != prev_model || self.hermes_provider != prev_provider {
                            hermes_save_after = true;
                        }
                        if self.hermes_cli_packet.is_some() {
                            ui.label(egui::RichText::new("[packet loaded]").small().color(UP));
                        }
                        let has_turns = !self.hermes_cli_history.is_empty();
                        ui.add_enabled_ui(has_turns, |ui| {
                            if ui
                                .button("\u{1F4BE} Save")
                                .on_hover_text("Export this Hermes session to a markdown file")
                                .clicked()
                            {
                                save_hermes_transcript = Some(Self::format_ai_transcript(
                                    &self.hermes_cli_history,
                                    "Hermes Agent CLI",
                                    "Hermes",
                                    Some(self.hermes_cli_session_id.as_str()),
                                ));
                            }
                            if ui
                                .button("\u{1F4E8} Matrix")
                                .on_hover_text("Post this Hermes session to the Community Chat room")
                                .clicked()
                            {
                                matrix_hermes_transcript = Some(Self::format_ai_transcript(
                                    &self.hermes_cli_history,
                                    "Hermes Agent CLI",
                                    "Hermes",
                                    Some(self.hermes_cli_session_id.as_str()),
                                ));
                            }
                        });
                    });
                    ui.separator();
                    let scroll_h = (ui.available_height() - 60.0).max(120.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(scroll_h)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            if self.hermes_cli_history.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(40.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "Ask Hermes anything — uses your local hermes CLI",
                                        )
                                        .color(AXIS_TEXT),
                                    );
                                });
                            }
                            for (is_user, msg) in &self.hermes_cli_history {
                                let (align, color, prefix) = if *is_user {
                                    (
                                        egui::Align::RIGHT,
                                        egui::Color32::from_rgb(80, 140, 255),
                                        "You",
                                    )
                                } else {
                                    (
                                        egui::Align::LEFT,
                                        egui::Color32::from_rgb(220, 180, 100),
                                        "Hermes",
                                    )
                                };
                                ui.with_layout(egui::Layout::top_down(align), |ui| {
                                    ui.label(
                                        egui::RichText::new(prefix).strong().small().color(color),
                                    );
                                    ui.label(egui::RichText::new(msg).small());
                                });
                                ui.add_space(4.0);
                            }
                            if self.hermes_cli_rx.is_some() {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(
                                        egui::RichText::new("Thinking...").small().color(AXIS_TEXT),
                                    );
                                });
                            }
                        });
                    ui.separator();
                    ui.horizontal(|ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.hermes_cli_input)
                                .desired_width(ui.available_width() - 60.0)
                                .hint_text("Ask Hermes..."),
                        );
                        let send = ui.button("Send").clicked()
                            || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                        if send
                            && !self.hermes_cli_input.trim().is_empty()
                            && self.hermes_cli_rx.is_none()
                        {
                            let msg = self.hermes_cli_input.trim().to_string();
                            self.hermes_cli_input.clear();
                            self.hermes_cli_history.push((true, msg.clone()));

                            let lower = msg.to_lowercase();
                            if lower == "/clear" {
                                self.hermes_cli_history.clear();
                                self.hermes_cli_history
                                    .push((false, "(chat history cleared)".to_string()));
                                self.hermes_cli_session_id.clear();
                                return;
                            }
                            if lower == "/help" {
                                self.hermes_cli_history.push((
                                    false,
                                    "Hermes chat help:\n\
                                     • Type any prompt and press Enter to ask Hermes Agent.\n\
                                     • ASKHERMES SYM[,SYM2] question — preload a TyphooN research packet.\n\
                                     • Optional Model/Provider fields map to hermes --model / --provider.\n\
                                     • /clear — clear this window's transcript\n\
                                     • /status — show local Hermes status"
                                        .to_string(),
                                ));
                                return;
                            }
                            if lower == "/status" {
                                let count = self.hermes_cli_history.iter().filter(|(u, _)| *u).count();
                                let has_pkt = if self.hermes_cli_packet.is_some() { "yes" } else { "no" };
                                self.hermes_cli_history.push((
                                    false,
                                    format!(
                                        "Hermes status:\n\
                                         • Backend: `hermes --oneshot` subprocess\n\
                                         • Model override: {}\n\
                                         • Provider override: {}\n\
                                         • Research packet loaded: {has_pkt}\n\
                                         • Messages this session: {count}",
                                        if self.hermes_model.trim().is_empty() { "(default)" } else { self.hermes_model.as_str() },
                                        if self.hermes_provider.trim().is_empty() { "(default)" } else { self.hermes_provider.as_str() },
                                    ),
                                ));
                                return;
                            }

                            let full_prompt = Self::build_claude_prompt(
                                self.hermes_cli_packet.as_deref(),
                                &self.hermes_cli_history,
                                &msg,
                                "",
                            );
                            let model = self.hermes_model.clone();
                            let provider = self.hermes_provider.clone();
                            let (tx, rx) = std::sync::mpsc::channel();
                            self.hermes_cli_rx = Some(rx);
                            Self::spawn_hermes_exec(model, provider, full_prompt, tx);
                        }
                    });
                });
            if let Some(t) = save_hermes_transcript {
                self.save_ai_session_to_file("hermes_cli", &t);
            }
            if let Some(t) = matrix_hermes_transcript {
                self.send_ai_session_to_matrix("hermes_cli", &t);
            }
            if hermes_save_after {
                self.save_session();
            }
        }
    }

    pub(super) fn render_ai_sessions_window(&mut self, ctx: &egui::Context) {
        if self.show_ai_sessions {
            // Auto-refresh index every 10s while the window is open.
            let now_ts = chrono::Utc::now().timestamp();
            if now_ts - self.ai_sessions_last_refresh > 10 {
                if let Some(ref cache) = self.cache {
                    self.ai_sessions_index =
                        typhoon_engine::core::ai_sessions::read_index(cache).unwrap_or_default();
                }
                self.ai_sessions_last_refresh = now_ts;
            }
            let mut browser_save: Option<(String, String)> = None;
            let mut browser_matrix: Option<(String, String)> = None;
            egui::Window::new("AI Sessions")
                .open(&mut self.show_ai_sessions)
                .resizable(true).default_size([760.0, 520.0])
                .min_width(520.0).min_height(320.0)
                .constrain(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{} saved sessions", self.ai_sessions_index.len())).small().color(AXIS_TEXT));
                        if ui.small_button("Refresh").clicked() {
                            if let Some(ref cache) = self.cache {
                                self.ai_sessions_index = typhoon_engine::core::ai_sessions::read_index(cache)
                                    .unwrap_or_default();
                            }
                            self.ai_sessions_last_refresh = now_ts;
                        }
                        if self.ai_sessions_viewing.is_some() {
                            if ui.small_button("Close transcript").clicked() {
                                self.ai_sessions_viewing = None;
                            }
                        }
                    });
                    ui.separator();

                    if let Some(rec) = self.ai_sessions_viewing.clone() {
                        ui.label(egui::RichText::new(format!(
                            "{} · {} · {} turns · started {}",
                            rec.provider, rec.model, rec.turns.len(),
                            chrono::DateTime::from_timestamp(rec.started_at, 0)
                                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                .unwrap_or_else(|| "-".into()),
                        )).small().color(AXIS_TEXT));
                        ui.label(egui::RichText::new(format!("Subject: {}", rec.subject)).small());
                        ui.separator();
                        let scroll_h = (ui.available_height() - 30.0).max(120.0);
                        egui::ScrollArea::vertical().auto_shrink(false).max_height(scroll_h).show(ui, |ui| {
                            for (is_user, msg) in rec.turns.iter() {
                                let (color, prefix) = if *is_user {
                                    (egui::Color32::from_rgb(80, 140, 255), "You")
                                } else {
                                    (egui::Color32::from_rgb(220, 180, 100), rec.provider.as_str())
                                };
                                ui.label(egui::RichText::new(prefix).strong().small().color(color));
                                ui.label(egui::RichText::new(msg).small());
                                ui.add_space(4.0);
                            }
                        });
                    } else {
                        let scroll_h = (ui.available_height() - 10.0).max(120.0);
                        egui::ScrollArea::vertical().auto_shrink(false).max_height(scroll_h).show(ui, |ui| {
                            egui::Grid::new("ai_sessions_grid").striped(true).num_columns(6).show(ui, |ui| {
                                ui.label(egui::RichText::new("Provider").strong().small());
                                ui.label(egui::RichText::new("Subject").strong().small());
                                ui.label(egui::RichText::new("Turns").strong().small());
                                ui.label(egui::RichText::new("Model").strong().small());
                                ui.label(egui::RichText::new("Last touched").strong().small());
                                ui.label("");
                                ui.end_row();

                                // Clone entries so we can mutate self.ai_sessions_viewing/etc. inside the loop.
                                let entries = self.ai_sessions_index.clone();
                                for entry in entries.iter() {
                                    ui.label(egui::RichText::new(&entry.provider).small());
                                    let subj = if entry.subject.is_empty() { "(no subject)" } else { entry.subject.as_str() };
                                    ui.label(egui::RichText::new(subj).small());
                                    ui.label(egui::RichText::new(format!("{}", entry.turn_count)).small());
                                    ui.label(egui::RichText::new(&entry.model).small());
                                    let ts = chrono::DateTime::from_timestamp(entry.last_touched_at, 0)
                                        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                        .unwrap_or_else(|| "-".into());
                                    ui.label(egui::RichText::new(ts).small());
                                    ui.horizontal(|ui| {
                                        if ui.small_button("View").clicked() {
                                            if let Some(ref cache) = self.cache {
                                                if let Ok(Some(rec)) = typhoon_engine::core::ai_sessions::load_session(
                                                    cache, &entry.provider, &entry.session_id,
                                                ) {
                                                    self.ai_sessions_viewing = Some(rec);
                                                }
                                            }
                                        }
                                        if ui.small_button("\u{1F4BE}").on_hover_text("Save session as markdown").clicked() {
                                            if let Some(ref cache) = self.cache {
                                                if let Ok(Some(rec)) = typhoon_engine::core::ai_sessions::load_session(
                                                    cache, &entry.provider, &entry.session_id,
                                                ) {
                                                    let t = Self::format_ai_transcript(
                                                        &rec.turns, &rec.provider, &rec.provider,
                                                        Some(rec.session_id.as_str()),
                                                    );
                                                    browser_save = Some((format!("{}_history", rec.provider), t));
                                                }
                                            }
                                        }
                                        if ui.small_button("\u{1F4E8}").on_hover_text("Send session to Matrix").clicked() {
                                            if let Some(ref cache) = self.cache {
                                                if let Ok(Some(rec)) = typhoon_engine::core::ai_sessions::load_session(
                                                    cache, &entry.provider, &entry.session_id,
                                                ) {
                                                    let t = Self::format_ai_transcript(
                                                        &rec.turns, &rec.provider, &rec.provider,
                                                        Some(rec.session_id.as_str()),
                                                    );
                                                    browser_matrix = Some((format!("{}_history", rec.provider), t));
                                                }
                                            }
                                        }
                                        if ui.small_button("Resume").clicked() {
                                            if let Some(ref cache) = self.cache {
                                                if let Ok(Some(rec)) = typhoon_engine::core::ai_sessions::load_session(
                                                    cache, &entry.provider, &entry.session_id,
                                                ) {
                                                    match entry.provider.as_str() {
                                                        "claude" => {
                                                            self.claude_code_history = rec.turns.clone();
                                                            self.claude_code_session_id = if rec.cli_session_id.is_empty() {
                                                                Some(rec.session_id.clone())
                                                            } else {
                                                                Some(rec.cli_session_id.clone())
                                                            };
                                                            self.show_claude_code = true;
                                                        }
                                                        "gemini" => {
                                                            self.gemini_cli_history = rec.turns.clone();
                                                            self.gemini_cli_session_id = rec.session_id.clone();
                                                            self.show_gemini_cli = true;
                                                        }
                                                        "codex" => {
                                                            self.codex_cli_history = rec.turns.clone();
                                                            self.codex_cli_session_id = rec.session_id.clone();
                                                            if !rec.model.trim().is_empty() {
                                                                self.codex_model = rec.model.clone();
                                                            }
                                                            self.show_codex_cli = true;
                                                        }
                                                        "ai_chat" => {
                                                            self.ai_chat_history = rec.turns.clone();
                                                            self.ai_chat_session_id = rec.session_id.clone();
                                                            self.show_ai_chat = true;
                                                        }
                                                        _ => {}
                                                    }
                                                    self.log.push_back(LogEntry::info(format!(
                                                        "Resumed {} session {} ({} turns)",
                                                        entry.provider, entry.session_id, rec.turns.len())));
                                                }
                                            }
                                        }
                                    });
                                    ui.end_row();
                                }
                            });
                        });
                    }
                });
            if let Some((slug, t)) = browser_save {
                self.save_ai_session_to_file(&slug, &t);
            }
            if let Some((slug, t)) = browser_matrix {
                self.send_ai_session_to_matrix(&slug, &t);
            }
        }
    }

    pub(super) fn render_ai_cache_window(&mut self, ctx: &egui::Context) {
        if self.show_ai_cache {
            let now_ts = chrono::Utc::now().timestamp();
            if now_ts - self.ai_cache_last_refresh > 10 {
                if let Some(ref cache) = self.cache {
                    self.ai_cache_stats =
                        typhoon_engine::core::ai_response_cache::stats(cache).unwrap_or_default();
                    self.ai_cache_recent =
                        typhoon_engine::core::ai_response_cache::recent_entries(cache, 50)
                            .unwrap_or_default();
                }
                self.ai_cache_last_refresh = now_ts;
            }
            egui::Window::new("AI Response Cache (ADR-162)")
                .open(&mut self.show_ai_cache)
                .resizable(true)
                .default_size([820.0, 560.0])
                .min_width(560.0)
                .min_height(340.0)
                .constrain(true)
                .show(ctx, |ui| {
                    let s = self.ai_cache_stats.clone();
                    ui.horizontal(|ui| {
                        if ui.small_button("Refresh").clicked() {
                            if let Some(ref cache) = self.cache {
                                self.ai_cache_stats =
                                    typhoon_engine::core::ai_response_cache::stats(cache)
                                        .unwrap_or_default();
                                self.ai_cache_recent =
                                    typhoon_engine::core::ai_response_cache::recent_entries(
                                        cache, 50,
                                    )
                                    .unwrap_or_default();
                            }
                            self.ai_cache_last_refresh = now_ts;
                        }
                        if ui.small_button("Prune >30d").clicked() {
                            if let Some(ref cache) = self.cache {
                                let days_30_secs: i64 = 30 * 24 * 3600;
                                match typhoon_engine::core::ai_response_cache::prune_older_than(
                                    cache,
                                    days_30_secs,
                                ) {
                                    Ok(n) => self.log.push_back(LogEntry::info(format!(
                                        "AICACHE: pruned {} entries older than 30 days",
                                        n
                                    ))),
                                    Err(e) => self
                                        .log
                                        .push_back(LogEntry::err(format!("AICACHE prune: {e}"))),
                                }
                                self.ai_cache_stats =
                                    typhoon_engine::core::ai_response_cache::stats(cache)
                                        .unwrap_or_default();
                                self.ai_cache_recent =
                                    typhoon_engine::core::ai_response_cache::recent_entries(
                                        cache, 50,
                                    )
                                    .unwrap_or_default();
                            }
                        }
                    });
                    ui.separator();
                    egui::Grid::new("ai_cache_stats_grid")
                        .striped(true)
                        .num_columns(2)
                        .min_col_width(220.0)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Entries").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{}", s.entry_count))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(
                                egui::RichText::new("Total hits (token-saving reuse)")
                                    .small()
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{}", s.total_hits))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(
                                egui::RichText::new("Prompt tokens saved (est.)")
                                    .small()
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{}", s.tokens_saved_prompt))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(
                                egui::RichText::new("Completion tokens saved (est.)")
                                    .small()
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{}", s.tokens_saved_completion))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(
                                egui::RichText::new("Total tokens saved (est.)")
                                    .small()
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}",
                                    s.tokens_saved_prompt + s.tokens_saved_completion
                                ))
                                .small()
                                .monospace(),
                            );
                            ui.end_row();
                            let oldest = if s.oldest_created_at > 0 {
                                chrono::DateTime::from_timestamp(s.oldest_created_at, 0)
                                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                    .unwrap_or_else(|| "-".into())
                            } else {
                                "-".into()
                            };
                            let newest = if s.newest_updated_at > 0 {
                                chrono::DateTime::from_timestamp(s.newest_updated_at, 0)
                                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                    .unwrap_or_else(|| "-".into())
                            } else {
                                "-".into()
                            };
                            ui.label(egui::RichText::new("Oldest entry").small().strong());
                            ui.label(egui::RichText::new(oldest).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Newest activity").small().strong());
                            ui.label(egui::RichText::new(newest).small().monospace());
                            ui.end_row();
                        });
                    if !s.providers.is_empty() {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("By provider:").small().strong());
                        for (prov, count) in s.providers.iter() {
                            ui.label(
                                egui::RichText::new(format!("  · {} — {}", prov, count))
                                    .small()
                                    .monospace(),
                            );
                        }
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!(
                            "Recent entries (newest first, {} shown):",
                            self.ai_cache_recent.len()
                        ))
                        .small()
                        .strong(),
                    );
                    let scroll_h = (ui.available_height() - 10.0).max(120.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(scroll_h)
                        .show(ui, |ui| {
                            egui::Grid::new("ai_cache_recent_grid")
                                .striped(true)
                                .num_columns(5)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("Provider").strong().small());
                                    ui.label(egui::RichText::new("Model").strong().small());
                                    ui.label(egui::RichText::new("Hits").strong().small());
                                    ui.label(egui::RichText::new("Last used").strong().small());
                                    ui.label(
                                        egui::RichText::new("Prompt preview").strong().small(),
                                    );
                                    ui.end_row();
                                    for e in self.ai_cache_recent.iter() {
                                        ui.label(
                                            egui::RichText::new(&e.provider).small().monospace(),
                                        );
                                        ui.label(egui::RichText::new(&e.model).small().monospace());
                                        ui.label(
                                            egui::RichText::new(format!("{}", e.hit_count))
                                                .small()
                                                .monospace(),
                                        );
                                        let ts = chrono::DateTime::from_timestamp(e.updated_at, 0)
                                            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                            .unwrap_or_else(|| "-".into());
                                        ui.label(egui::RichText::new(ts).small().monospace());
                                        let preview = if e.prompt_preview.len() > 80 {
                                            format!(
                                                "{}…",
                                                &e.prompt_preview
                                                    .chars()
                                                    .take(80)
                                                    .collect::<String>()
                                            )
                                        } else {
                                            e.prompt_preview.clone()
                                        };
                                        ui.label(egui::RichText::new(preview).small());
                                        ui.end_row();
                                    }
                                });
                        });
                });
        }
    }
}
