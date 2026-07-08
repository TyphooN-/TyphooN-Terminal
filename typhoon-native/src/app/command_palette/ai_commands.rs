use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_ai_command(&mut self, cmd_upper: &str) -> bool {
        match cmd_upper {
            "AI" | "AI_CHAT" | "ASKAI" | "ASK_AI" | "INVESTIGATE" => self.show_ai_chat = true,
            "ANTIGRAVITY" | "ANTIGRAVITY_CLI" | "ANTIGRAVITY-CLI" | "ASKANTIGRAVITY"
            | "ASK_ANTIGRAVITY" | "GEMINI" | "GEMINI_CLI" | "GEMINI-CLI" | "ASKGEMINI"
            | "ASK_GEMINI" => {
                let tool = Self::google_ai_cli_binary();
                if Self::google_ai_cli_available() {
                    self.show_gemini_cli = true;
                    self.log.push_back(LogEntry::info(format!(
                        "{} CLI detected — opening Google AI chat",
                        if tool == "antigravity" {
                            "Antigravity"
                        } else {
                            "Gemini"
                        }
                    )));
                } else {
                    self.log.push_back(LogEntry::err("Antigravity/Gemini CLI not found in PATH. Install Antigravity CLI (preferred) or Gemini CLI."));
                }
            }
            "CLAUDE" | "CLAUDE_CODE" | "CLAUDE-CODE" | "ASKCLAUDE" | "ASK_CLAUDE" => {
                // Check if claude binary exists
                match std::process::Command::new("which").arg("claude").output() {
                    Ok(out) if out.status.success() => {
                        self.show_claude_code = true;
                        self.log
                            .push_back(LogEntry::info("Claude Code CLI detected — opening chat"));
                    }
                    _ => {
                        self.log.push_back(LogEntry::err("Claude Code CLI not found in PATH. Install: npm install -g @anthropic-ai/claude-code"));
                    }
                }
            }
            "CODEX" | "CODEX_CLI" | "CODEX-CLI" | "ASKCODEX" | "ASK_CODEX" => {
                match std::process::Command::new("which").arg("codex").output() {
                    Ok(out) if out.status.success() => {
                        self.show_codex_cli = true;
                        self.log
                            .push_back(LogEntry::info("Codex CLI detected — opening chat"));
                    }
                    _ => {
                        self.log.push_back(LogEntry::err(
                            "Codex CLI not found in PATH. Install: npm install -g @openai/codex",
                        ));
                    }
                }
            }
            "HERMES" | "HERMES_CLI" | "HERMES-CLI" | "ASKHERMES" | "ASK_HERMES" => {
                match std::process::Command::new("which").arg("hermes").output() {
                    Ok(out) if out.status.success() => {
                        self.show_hermes_cli = true;
                        self.log
                            .push_back(LogEntry::info("Hermes Agent CLI detected — opening chat"));
                    }
                    _ => {
                        self.log.push_back(LogEntry::err(
                            "Hermes Agent CLI not found in PATH. Install/configure Hermes Agent first.",
                        ));
                    }
                }
            }
            "GROK" | "GROK_CLI" | "GROK-BUILD" | "GROK_BUILD" | "ASKGROK" | "ASK_GROK" => {
                match std::process::Command::new("which").arg("grok").output() {
                    Ok(out) if out.status.success() => {
                        self.show_grok_cli = true;
                        self.log
                            .push_back(LogEntry::info("Grok Build CLI detected — opening chat"));
                    }
                    _ => {
                        self.log.push_back(LogEntry::err(
                            "Grok Build CLI not found in PATH. Install/configure the grok binary first.",
                        ));
                    }
                }
            }
            // ── AI session resume + history browser ──
            "RESUMECLAUDE" | "RESUME_CLAUDE" => {
                if let Some(ref cache) = self.cache {
                    match typhoon_engine::core::ai_sessions::latest_for_provider(cache, "claude") {
                        Ok(Some(rec)) => {
                            self.claude_code_history = rec.turns.clone();
                            self.claude_code_session_id = if rec.cli_session_id.is_empty() {
                                Some(rec.session_id.clone())
                            } else {
                                Some(rec.cli_session_id.clone())
                            };
                            self.show_claude_code = true;
                            self.log.push_back(LogEntry::info(format!(
                                "Resumed Claude session {} ({} turns)",
                                rec.session_id,
                                rec.turns.len()
                            )));
                        }
                        Ok(None) => self
                            .log
                            .push_back(LogEntry::warn("No saved Claude session to resume")),
                        Err(e) => self
                            .log
                            .push_back(LogEntry::err(format!("RESUMECLAUDE: {e}"))),
                    }
                } else {
                    self.log
                        .push_back(LogEntry::warn("Cache not ready — wait a moment and retry"));
                }
            }
            "RESUMEGEMINI" | "RESUME_GEMINI" => {
                if let Some(ref cache) = self.cache {
                    match typhoon_engine::core::ai_sessions::latest_for_provider(cache, "gemini") {
                        Ok(Some(rec)) => {
                            self.gemini_cli_history = rec.turns.clone();
                            self.gemini_cli_session_id = rec.session_id.clone();
                            self.show_gemini_cli = true;
                            self.log.push_back(LogEntry::info(format!(
                                "Resumed Gemini session {} ({} turns — no native --resume, transcript replayed as context)",
                                rec.session_id, rec.turns.len())));
                        }
                        Ok(None) => self
                            .log
                            .push_back(LogEntry::warn("No saved Gemini session to resume")),
                        Err(e) => self
                            .log
                            .push_back(LogEntry::err(format!("RESUMEGEMINI: {e}"))),
                    }
                } else {
                    self.log
                        .push_back(LogEntry::warn("Cache not ready — wait a moment and retry"));
                }
            }
            "RESUMECODEX" | "RESUME_CODEX" => {
                if let Some(ref cache) = self.cache {
                    match typhoon_engine::core::ai_sessions::latest_for_provider(cache, "codex") {
                        Ok(Some(rec)) => {
                            self.codex_cli_history = rec.turns.clone();
                            self.codex_cli_session_id = rec.session_id.clone();
                            self.show_codex_cli = true;
                            self.log.push_back(LogEntry::info(format!(
                                "Resumed Codex session {} ({} turns — no native resume, transcript replayed as context)",
                                rec.session_id, rec.turns.len())));
                        }
                        Ok(None) => self
                            .log
                            .push_back(LogEntry::warn("No saved Codex session to resume")),
                        Err(e) => self
                            .log
                            .push_back(LogEntry::err(format!("RESUMECODEX: {e}"))),
                    }
                } else {
                    self.log
                        .push_back(LogEntry::warn("Cache not ready — wait a moment and retry"));
                }
            }
            "RESUMEHERMES" | "RESUME_HERMES" => {
                if let Some(ref cache) = self.cache {
                    match typhoon_engine::core::ai_sessions::latest_for_provider(cache, "hermes") {
                        Ok(Some(rec)) => {
                            self.hermes_cli_history = rec.turns.clone();
                            self.hermes_cli_session_id = rec.session_id.clone();
                            self.show_hermes_cli = true;
                            self.log.push_back(LogEntry::info(format!(
                                "Resumed Hermes session {} ({} turns — transcript replayed as context)",
                                rec.session_id, rec.turns.len())));
                        }
                        Ok(None) => self
                            .log
                            .push_back(LogEntry::warn("No saved Hermes session to resume")),
                        Err(e) => self
                            .log
                            .push_back(LogEntry::err(format!("RESUMEHERMES: {e}"))),
                    }
                } else {
                    self.log
                        .push_back(LogEntry::warn("Cache not ready — wait a moment and retry"));
                }
            }
            "RESUMEGROK" | "RESUME_GROK" => {
                if let Some(ref cache) = self.cache {
                    match typhoon_engine::core::ai_sessions::latest_for_provider(cache, "grok") {
                        Ok(Some(rec)) => {
                            self.grok_cli_history = rec.turns.clone();
                            self.grok_cli_session_id = rec.session_id.clone();
                            self.show_grok_cli = true;
                            self.log.push_back(LogEntry::info(format!(
                                "Resumed Grok session {} ({} turns — transcript replayed as context)",
                                rec.session_id, rec.turns.len())));
                        }
                        Ok(None) => self
                            .log
                            .push_back(LogEntry::warn("No saved Grok session to resume")),
                        Err(e) => self
                            .log
                            .push_back(LogEntry::err(format!("RESUMEGROK: {e}"))),
                    }
                } else {
                    self.log
                        .push_back(LogEntry::warn("Cache not ready — wait a moment and retry"));
                }
            }
            "RESUMEAI" | "RESUME_AI" => {
                if let Some(ref cache) = self.cache {
                    match typhoon_engine::core::ai_sessions::latest_for_provider(cache, "ai_chat") {
                        Ok(Some(rec)) => {
                            self.ai_chat_history = rec.turns.clone();
                            self.ai_chat_session_id = rec.session_id.clone();
                            self.show_ai_chat = true;
                            self.log.push_back(LogEntry::info(format!(
                                "Resumed AI chat session {} ({} turns)",
                                rec.session_id,
                                rec.turns.len()
                            )));
                        }
                        Ok(None) => self
                            .log
                            .push_back(LogEntry::warn("No saved AI chat session to resume")),
                        Err(e) => self.log.push_back(LogEntry::err(format!("RESUMEAI: {e}"))),
                    }
                } else {
                    self.log
                        .push_back(LogEntry::warn("Cache not ready — wait a moment and retry"));
                }
            }
            "AISESSIONS" | "AI_SESSIONS" => {
                self.show_ai_sessions = true;
                if let Some(ref cache) = self.cache {
                    self.ai_sessions_index =
                        typhoon_engine::core::ai_sessions::read_index(cache).unwrap_or_default();
                }
                self.ai_sessions_last_refresh = chrono::Utc::now().timestamp();
            }
            "SCREENSHOTS" | "GALLERY" => {
                self.show_screenshots_gallery = true;
                self.scan_screenshots();
            }
            // ── cross-client AI response cache stats ──
            "AICACHE" | "AI_CACHE" | "AI_RESPONSE_CACHE" | "RESPONSE_CACHE" => {
                self.show_ai_cache = true;
                if let Some(ref cache) = self.cache {
                    self.ai_cache_stats =
                        typhoon_engine::core::ai_response_cache::stats(cache).unwrap_or_default();
                    self.ai_cache_recent =
                        typhoon_engine::core::ai_response_cache::recent_entries(cache, 50)
                            .unwrap_or_default();
                }
                self.ai_cache_last_refresh = chrono::Utc::now().timestamp();
            }
            // Investigation variants — open the window AND pre-load a research packet for the given symbols.
            cmd if cmd.starts_with("ASKAI ")
                || cmd.starts_with("ASK_AI ")
                || cmd.starts_with("INVESTIGATE ") =>
            {
                let args = cmd
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap_or("")
                    .trim();
                let (syms, question) = Self::parse_ask_args(args);
                if syms.is_empty() {
                    self.show_ai_chat = true;
                    self.log.push_back(LogEntry::warn(
                        "Usage: ASKAI SYM1[,SYM2] [optional question]",
                    ));
                } else {
                    let packet = self.investigate_symbols(&syms, &question);
                    // Persist the packet so follow-up Sends still see the fundamentals
                    // (not just a "[Research packet: …]" placeholder in the history).
                    self.ai_chat_packet = Some(packet.clone());
                    self.show_ai_chat = true;
                    self.ai_chat_history.push((
                        true,
                        format!(
                            "[Research packet loaded: {}] {}",
                            syms.join(", "),
                            if question.is_empty() {
                                "Give me an overall read on these tickers.".to_string()
                            } else {
                                question.clone()
                            }
                        ),
                    ));
                    let first_turn = if question.is_empty() {
                        "Give me an overall read on these tickers — combine the research packet with live web search for recent news/sentiment.".to_string()
                    } else {
                        question.clone()
                    };
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
                            message: first_turn,
                            history: Vec::new(), // fresh chain — packet is in the system prompt
                            system: Some(packet),
                            model: Some(self.ai_model.clone()),
                        });
                        self.log.push_back(LogEntry::info(format!(
                            "AI investigation dispatched: {} ({} symbols, {} backend, {})",
                            syms.join(", "),
                            syms.len(),
                            provider,
                            self.ai_model
                        )));
                    }
                }
            }
            cmd if cmd.starts_with("ASKCLAUDE ") || cmd.starts_with("ASK_CLAUDE ") => {
                let args = cmd
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap_or("")
                    .trim();
                let (syms, question) = Self::parse_ask_args(args);
                if syms.is_empty() {
                    self.show_claude_code = true;
                    self.log.push_back(LogEntry::warn(
                        "Usage: ASKCLAUDE SYM1[,SYM2] [optional question]",
                    ));
                    return true;
                }
                match std::process::Command::new("which").arg("claude").output() {
                    Ok(out) if out.status.success() => {
                        let packet = self.investigate_symbols(&syms, &question);
                        // Store the packet so follow-ups in the Claude Code window still
                        // have access to the same research context. `build_claude_prompt`
                        // re-injects it on every Send.
                        self.claude_code_packet = Some(packet.clone());
                        self.show_claude_code = true;
                        let first_user_turn = if question.is_empty() {
                            format!(
                                "Give me an overall read on {} — combine the research packet above with a live web search for recent news/sentiment.",
                                syms.join(", ")
                            )
                        } else {
                            question.clone()
                        };
                        self.claude_code_history.push((
                            true,
                            format!(
                                "[Research packet loaded: {}] {}",
                                syms.join(", "),
                                first_user_turn
                            ),
                        ));
                        if self.claude_code_rx.is_none() {
                            // Fresh session UUID — subsequent Sends in the window will --resume.
                            let session_id = Self::new_uuid();
                            self.claude_code_session_id = Some(session_id.clone());
                            let model = self.claude_model.clone();
                            let effort = self.claude_effort.clone();
                            let full_prompt = Self::build_claude_prompt(
                                Some(&packet),
                                &self.claude_code_history,
                                &first_user_turn,
                                &self.claude_effort,
                            );
                            let (tx, rx) = std::sync::mpsc::channel();
                            self.claude_code_rx = Some(rx);
                            Self::spawn_claude_print(
                                model,
                                effort,
                                session_id,
                                true,
                                full_prompt,
                                tx,
                            );
                            self.log.push_back(LogEntry::info(format!(
                                "Claude Code investigation dispatched: {} ({} symbols, {} model)",
                                syms.join(", "),
                                syms.len(),
                                self.claude_model
                            )));
                        }
                    }
                    _ => {
                        self.log
                            .push_back(LogEntry::err("Claude Code CLI not found in PATH."));
                    }
                }
            }
            cmd if cmd.starts_with("ASKANTIGRAVITY ")
                || cmd.starts_with("ASK_ANTIGRAVITY ")
                || cmd.starts_with("ASKGEMINI ")
                || cmd.starts_with("ASK_GEMINI ") =>
            {
                let args = cmd
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap_or("")
                    .trim();
                let (syms, question) = Self::parse_ask_args(args);
                if syms.is_empty() {
                    self.show_gemini_cli = true;
                    self.log.push_back(LogEntry::warn(
                        "Usage: ASKANTIGRAVITY SYM1[,SYM2] [optional question]",
                    ));
                    return true;
                }
                let tool = Self::google_ai_cli_binary();
                if Self::google_ai_cli_available() {
                    let packet = self.investigate_symbols(&syms, &question);
                    self.gemini_cli_packet = Some(packet.clone());
                    self.show_gemini_cli = true;
                    let first_user_turn = if question.is_empty() {
                        format!(
                            "Give me an overall read on {} — combine the research packet above with a live web search for recent news/sentiment.",
                            syms.join(", ")
                        )
                    } else {
                        question.clone()
                    };
                    self.gemini_cli_history.push((
                        true,
                        format!(
                            "[Research packet loaded: {}] {}",
                            syms.join(", "),
                            first_user_turn
                        ),
                    ));
                    if self.gemini_cli_rx.is_none() {
                        let model = self.gemini_model.clone();
                        let full_prompt = Self::build_claude_prompt(
                            Some(&packet),
                            &self.gemini_cli_history,
                            &first_user_turn,
                            "",
                        );
                        let (tx, rx) = std::sync::mpsc::channel();
                        self.gemini_cli_rx = Some(rx);
                        Self::spawn_gemini_prompt(model, full_prompt, tx);
                        self.log.push_back(LogEntry::info(format!(
                            "{} CLI investigation dispatched: {} ({} symbols, {})",
                            if tool == "antigravity" {
                                "Antigravity"
                            } else {
                                "Gemini"
                            },
                            syms.join(", "),
                            syms.len(),
                            self.gemini_model
                        )));
                    }
                } else {
                    self.log
                        .push_back(LogEntry::err("Antigravity/Gemini CLI not found in PATH."));
                }
            }
            cmd if cmd.starts_with("ASKHERMES ") || cmd.starts_with("ASK_HERMES ") => {
                let args = cmd
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap_or("")
                    .trim();
                let (syms, question) = Self::parse_ask_args(args);
                if syms.is_empty() {
                    self.show_hermes_cli = true;
                    self.log.push_back(LogEntry::warn(
                        "Usage: ASKHERMES SYM1[,SYM2] [optional question]",
                    ));
                    return true;
                }
                match std::process::Command::new("which").arg("hermes").output() {
                    Ok(out) if out.status.success() => {
                        let packet = self.investigate_symbols(&syms, &question);
                        self.hermes_cli_packet = Some(packet.clone());
                        self.show_hermes_cli = true;
                        let first_user_turn = if question.is_empty() {
                            format!(
                                "Give me an overall read on {} — combine the research packet above with live web search for recent news/sentiment.",
                                syms.join(", ")
                            )
                        } else {
                            question.clone()
                        };
                        self.hermes_cli_history.push((
                            true,
                            format!(
                                "[Research packet loaded: {}] {}",
                                syms.join(", "),
                                first_user_turn
                            ),
                        ));
                        if self.hermes_cli_rx.is_none() {
                            let full_prompt = Self::build_claude_prompt(
                                Some(&packet),
                                &self.hermes_cli_history,
                                &first_user_turn,
                                "",
                            );
                            let model = self.hermes_model.clone();
                            let provider = self.hermes_provider.clone();
                            let (tx, rx) = std::sync::mpsc::channel();
                            self.hermes_cli_rx = Some(rx);
                            Self::spawn_hermes_exec(model, provider, full_prompt, tx);
                            self.log.push_back(LogEntry::info(format!(
                                "Hermes Agent investigation dispatched: {} ({} symbols{})",
                                syms.join(", "),
                                syms.len(),
                                if self.hermes_model.trim().is_empty() {
                                    "".to_string()
                                } else {
                                    format!(", {}", self.hermes_model)
                                }
                            )));
                        }
                    }
                    _ => {
                        self.log
                            .push_back(LogEntry::err("Hermes Agent CLI not found in PATH."));
                    }
                }
            }
            cmd if cmd.starts_with("ASKGROK ") || cmd.starts_with("ASK_GROK ") => {
                let args = cmd
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap_or("")
                    .trim();
                let (syms, question) = Self::parse_ask_args(args);
                if syms.is_empty() {
                    self.show_grok_cli = true;
                    self.log.push_back(LogEntry::warn(
                        "Usage: ASKGROK SYM1[,SYM2] [optional question]",
                    ));
                    return true;
                }
                match std::process::Command::new("which").arg("grok").output() {
                    Ok(out) if out.status.success() => {
                        let packet = self.investigate_symbols(&syms, &question);
                        self.grok_cli_packet = Some(packet.clone());
                        self.show_grok_cli = true;
                        let first_user_turn = if question.is_empty() {
                            format!(
                                "Give me an overall read on {} — combine the research packet above with live web search for recent news/sentiment.",
                                syms.join(", ")
                            )
                        } else {
                            question.clone()
                        };
                        self.grok_cli_history.push((
                            true,
                            format!(
                                "[Research packet loaded: {}] {}",
                                syms.join(", "),
                                first_user_turn
                            ),
                        ));
                        if self.grok_cli_rx.is_none() {
                            let model = self.grok_model.clone();
                            let effort = self.grok_effort.clone();
                            let full_prompt = Self::build_claude_prompt(
                                Some(&packet),
                                &self.grok_cli_history,
                                &first_user_turn,
                                "",
                            );
                            let (tx, rx) = std::sync::mpsc::channel();
                            self.grok_cli_rx = Some(rx);
                            Self::spawn_grok_exec(model, effort, full_prompt, tx);
                            self.log.push_back(LogEntry::info(format!(
                                "Grok Build investigation dispatched: {} ({} symbols, model {}, effort {})",
                                syms.join(", "),
                                syms.len(),
                                if self.grok_model.trim().is_empty() { "auto" } else { self.grok_model.as_str() },
                                Self::grok_effort_label(&self.grok_effort)
                            )));
                        }
                    }
                    _ => {
                        self.log
                            .push_back(LogEntry::err("Grok Build CLI not found in PATH."));
                    }
                }
            }
            cmd if cmd.starts_with("ASKCODEX ") || cmd.starts_with("ASK_CODEX ") => {
                let args = cmd
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap_or("")
                    .trim();
                let (syms, question) = Self::parse_ask_args(args);
                if syms.is_empty() {
                    self.show_codex_cli = true;
                    self.log.push_back(LogEntry::warn(
                        "Usage: ASKCODEX SYM1[,SYM2] [optional question]",
                    ));
                    return true;
                }
                match std::process::Command::new("which").arg("codex").output() {
                    Ok(out) if out.status.success() => {
                        let packet = self.investigate_symbols(&syms, &question);
                        self.codex_cli_packet = Some(packet.clone());
                        self.show_codex_cli = true;
                        let first_user_turn = if question.is_empty() {
                            format!(
                                "Give me an overall read on {} — combine the research packet above with a live web search for recent news/sentiment.",
                                syms.join(", ")
                            )
                        } else {
                            question.clone()
                        };
                        self.codex_cli_history.push((
                            true,
                            format!(
                                "[Research packet loaded: {}] {}",
                                syms.join(", "),
                                first_user_turn
                            ),
                        ));
                        if self.codex_cli_rx.is_none() {
                            let model = self.codex_model.clone();
                            let reasoning_effort = self.codex_reasoning_effort.clone();
                            let full_prompt = Self::build_claude_prompt(
                                Some(&packet),
                                &self.codex_cli_history,
                                &first_user_turn,
                                "",
                            );
                            let (tx, rx) = std::sync::mpsc::channel();
                            self.codex_cli_rx = Some(rx);
                            Self::spawn_codex_exec(model, reasoning_effort, full_prompt, tx);
                            self.log.push_back(LogEntry::info(format!(
                                "Codex CLI investigation dispatched: {} ({} symbols, {}, {})",
                                syms.join(", "),
                                syms.len(),
                                self.codex_model,
                                Self::codex_reasoning_effort_label(&self.codex_reasoning_effort)
                            )));
                        }
                    }
                    _ => {
                        self.log
                            .push_back(LogEntry::err("Codex CLI not found in PATH."));
                    }
                }
            }
            // Export the assembled research packet to a Markdown file (no AI
            // dispatch). Same arg parser + packet builder as the ASK commands —
            // this is the "packet half" of ASK without sending it to a model.
            "EXPORT_PACKET" | "EXPORTPACKET" | "PACKET_EXPORT" | "SAVE_PACKET" | "SAVEPACKET" => {
                self.log.push_back(LogEntry::warn(
                    "Usage: EXPORT_PACKET SYM1[,SYM2] [optional question]",
                ));
            }
            cmd if cmd.starts_with("EXPORT_PACKET ")
                || cmd.starts_with("EXPORTPACKET ")
                || cmd.starts_with("PACKET_EXPORT ")
                || cmd.starts_with("SAVE_PACKET ")
                || cmd.starts_with("SAVEPACKET ") =>
            {
                let args = cmd
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap_or("")
                    .trim();
                let (syms, question) = Self::parse_ask_args(args);
                if syms.is_empty() {
                    self.log.push_back(LogEntry::warn(
                        "Usage: EXPORT_PACKET SYM1[,SYM2] [optional question]",
                    ));
                    return true;
                }
                let packet = self.investigate_symbols(&syms, &question);
                let default_name = format!(
                    "{}_research_packet_{}.md",
                    Self::packet_export_stem(&syms),
                    chrono::Utc::now().format("%Y%m%d-%H%M%S")
                );
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Markdown", &["md"])
                    .set_file_name(&default_name)
                    .set_title("Export Research Packet")
                    .save_file()
                {
                    match std::fs::write(&path, &packet) {
                        Ok(()) => {
                            self.log.push_back(LogEntry::info(format!(
                                "Research packet exported: {} ({} symbols, {} bytes) → {}",
                                syms.join(", "),
                                syms.len(),
                                packet.len(),
                                path.display()
                            )));
                        }
                        Err(e) => {
                            self.log.push_back(LogEntry::err(format!(
                                "Research packet export failed: {}",
                                e
                            )));
                        }
                    }
                } else {
                    self.log
                        .push_back(LogEntry::info("Research packet export cancelled"));
                }
            }
            _ => return false,
        }
        true
    }
}
