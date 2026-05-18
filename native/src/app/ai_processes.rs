use super::*;

impl TyphooNApp {
    pub(super) fn build_packet_tree(text: &str) -> Vec<PacketTreeNode> {
        let mut out = Vec::new();
        let mut offset: usize = 0;
        for line in text.split_inclusive('\n') {
            let trimmed = line.trim_start();
            let depth = if trimmed.starts_with("#### ") {
                4
            } else if trimmed.starts_with("### ") {
                3
            } else if trimmed.starts_with("## ") {
                2
            } else {
                0
            };
            if depth > 0 {
                let title = trimmed
                    .trim_start_matches('#')
                    .trim()
                    .trim_end_matches('\n')
                    .to_string();
                out.push(PacketTreeNode {
                    depth,
                    title,
                    byte_offset: offset,
                });
            }
            offset += line.len();
        }
        out
    }

    /// Parse the argument portion of an ASKAI/ASKCLAUDE/ASKGEMINI/ASKCODEX command.
    ///
    /// The contract is simple and predictable: the **first whitespace-separated
    /// token** is the comma-separated symbol list; **everything after the first
    /// whitespace** is the question, preserved verbatim.
    ///
    ///   ASKAI CC,NCLH                            -> syms=[CC, NCLH], q=""
    ///   ASKAI CC,NCLH what's their debt load?    -> syms=[CC, NCLH], q="what's their debt load?"
    ///   ASKAI CC what is the outlook?            -> syms=[CC],       q="what is the outlook?"
    ///
    /// Note: handle_command() has already uppercased `args` by the time we're
    /// called, so the question ends up uppercased in the returned string. That
    /// is fine — we use it as prompt text for an LLM, not for matching.
    /// Space-separated symbol lists (e.g. "CC NCLH") are NOT supported — use
    /// commas — because we cannot reliably distinguish a second symbol from
    /// the first word of an English question once everything is uppercase.
    pub(super) fn parse_ask_args(args: &str) -> (Vec<String>, String) {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            return (Vec::new(), String::new());
        }

        let mut split = trimmed.splitn(2, char::is_whitespace);
        let first = split.next().unwrap_or("");
        let question = split.next().unwrap_or("").trim().to_string();

        let is_tickerish = |s: &str| -> bool {
            !s.is_empty()
                && s.len() <= 15
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '+' | '/'))
        };

        let mut seen = std::collections::HashSet::new();
        let syms: Vec<String> = first
            .split(',')
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty() && is_tickerish(s))
            .filter(|s| seen.insert(s.clone()))
            .collect();

        (syms, question)
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(super) fn parse_ask_args_test(args: &str) -> (Vec<String>, String) {
        Self::parse_ask_args(args)
    }

    /// Persist one AI conversation turn to the kv_cache. No-op when the cache
    /// is not yet open (pre-load startup race) or the session_id is empty. Logs
    /// a warning on error but never returns one — this is a best-effort audit
    /// trail, not a critical path.
    pub(super) fn persist_ai_turn(
        &self,
        provider: &str,
        session_id: &str,
        cli_session_id: Option<&str>,
        history: &[(bool, String)],
        model: &str,
    ) {
        if session_id.trim().is_empty() || history.is_empty() {
            return;
        }
        if let Some(ref cache) = self.cache {
            if let Err(e) = typhoon_engine::core::ai_sessions::persist_turn(
                cache,
                session_id,
                provider,
                cli_session_id,
                history,
                model,
            ) {
                tracing::warn!("ai session persist {}/{}: {}", provider, session_id, e);
            }
        }
    }

    /// Queue ADR-130 Return Path ingestion when an AI reply includes a
    /// `===TYPHOON_INGEST===` block. The broker owns parsing and cache writes
    /// so manual paste, LAN remote ingest, and auto-ingest stay identical.
    pub(super) fn maybe_queue_ingest_from_ai_response(&mut self, agent: &str, response: &str) {
        if !response.contains("===TYPHOON_INGEST===") {
            return;
        }
        let agent_tag = agent.trim().to_lowercase();
        let _ = self.broker_tx.send(BrokerCmd::IngestResearchArticles {
            text: response.to_string(),
            agent_override: agent_tag.clone(),
        });
        self.log.push_back(LogEntry::info(format!(
            "AI Return Path ingest queued from {}",
            if agent_tag.is_empty() {
                "ai"
            } else {
                agent_tag.as_str()
            }
        )));
    }

    /// Ensure the given field has a UUID — used by the per-agent resume slash
    /// commands and by the first-turn auto-save in each reply-receipt site.
    pub(super) fn ensure_session_id(id: &mut String) -> String {
        if id.trim().is_empty() {
            *id = Self::new_uuid();
        }
        id.clone()
    }

    /// Generate a UUID-ish string for per-window Claude session tracking.
    /// Uses the system random source + nanos so collisions across restarts are
    /// effectively impossible. RFC 4122 v4 shape so Claude CLI accepts it.
    pub(super) fn new_uuid() -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id() as u128;
        let mut seed = nanos ^ (pid << 64);
        let mut bytes = [0u8; 16];
        for b in bytes.iter_mut() {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *b = (seed >> 33) as u8;
        }
        // RFC 4122 v4 bits.
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            bytes[0],
            bytes[1],
            bytes[2],
            bytes[3],
            bytes[4],
            bytes[5],
            bytes[6],
            bytes[7],
            bytes[8],
            bytes[9],
            bytes[10],
            bytes[11],
            bytes[12],
            bytes[13],
            bytes[14],
            bytes[15],
        )
    }

    /// Build the full Claude CLI prompt from a stored research packet, the
    /// visible chat history, and the user's latest message. The packet and
    /// transcript are prepended every call so follow-ups don't lose context
    /// between `claude --print` invocations.
    pub(super) fn build_claude_prompt(
        packet: Option<&str>,
        history: &[(bool, String)],
        latest: &str,
        effort: &str,
    ) -> String {
        let mut out = String::with_capacity(4096);
        if let Some(p) = packet {
            out.push_str("You have this TyphooN-Terminal research packet as background context. ");
            out.push_str("Use it to ground your answers; combine it with live web searches when the question needs recent news or prices.\n\n");
            out.push_str("=== RESEARCH PACKET ===\n");
            out.push_str(p);
            out.push_str("\n=== END RESEARCH PACKET ===\n\n");
        }
        // Prior turns excluding the just-pushed "latest" message (last entry in history).
        let prior: Vec<&(bool, String)> = history
            .iter()
            .take(history.len().saturating_sub(1))
            .filter(|(_, m)| !m.starts_with("[Research packet:"))
            .collect();
        if !prior.is_empty() {
            out.push_str("=== PRIOR CONVERSATION ===\n");
            for (is_user, m) in &prior {
                out.push_str(if *is_user { "User: " } else { "Assistant: " });
                out.push_str(m);
                out.push_str("\n\n");
            }
            out.push_str("=== END PRIOR CONVERSATION ===\n\n");
        }
        out.push_str("User: ");
        // Extended-thinking trigger: Claude Code CLI escalates thinking-token
        // budget based on magic phrases in the prompt (think < think hard <
        // think harder < ultrathink). Empty = no extended thinking.
        let eff = effort.trim();
        if !eff.is_empty() {
            out.push_str(eff);
            out.push_str(". ");
        }
        out.push_str(latest);
        out
    }

    /// Human-readable label for the current effort trigger (shown in the
    /// ComboBox selected_text and `/status`).
    pub(super) fn claude_effort_label(effort: &str) -> &'static str {
        match effort.trim() {
            "ultrathink" => "max (ultrathink)",
            "think harder" => "high (think harder)",
            "think hard" => "medium (think hard)",
            "think" => "low (think)",
            _ => "off",
        }
    }

    pub(super) fn normalize_codex_reasoning_effort(effort: &str) -> &'static str {
        match effort.trim() {
            "minimal" => "minimal",
            "low" => "low",
            "medium" => "medium",
            "high" => "high",
            "xhigh" => "xhigh",
            _ => "default",
        }
    }

    pub(super) fn codex_reasoning_effort_label(effort: &str) -> &'static str {
        match Self::normalize_codex_reasoning_effort(effort) {
            "minimal" => "minimal",
            "low" => "low",
            "medium" => "medium",
            "high" => "high",
            "xhigh" => "max (xhigh)",
            _ => "model default",
        }
    }

    pub(super) fn build_codex_exec_args(
        model: &str,
        reasoning_effort: &str,
        prompt: &str,
    ) -> Vec<String> {
        let mut args = vec![
            "exec".to_string(),
            "--model".to_string(),
            model.to_string(),
            "--skip-git-repo-check".to_string(),
        ];
        let effort = Self::normalize_codex_reasoning_effort(reasoning_effort);
        if effort != "default" {
            args.push("-c".to_string());
            args.push(format!("model_reasoning_effort=\"{}\"", effort));
        }
        args.push(prompt.to_string());
        args
    }

    pub(super) fn spawn_codex_exec(
        model: String,
        reasoning_effort: String,
        prompt: String,
        tx: std::sync::mpsc::Sender<String>,
    ) {
        std::thread::spawn(move || {
            let args = Self::build_codex_exec_args(&model, &reasoning_effort, &prompt);
            match std::process::Command::new("codex").args(&args).output() {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let response = if !stdout.trim().is_empty() {
                        stdout.trim().to_string()
                    } else if !stderr.trim().is_empty() {
                        format!("Error: {}", stderr.trim())
                    } else {
                        "(empty response)".to_string()
                    };
                    let _ = tx.send(response);
                }
                Err(e) => {
                    let _ = tx.send(format!("Failed to run codex CLI: {e}"));
                }
            }
        });
    }

    pub(super) fn build_hermes_exec_args(model: &str, provider: &str, prompt: &str) -> Vec<String> {
        let mut args = Vec::new();
        let model = model.trim();
        let provider = provider.trim();
        if !model.is_empty() {
            args.push("--model".to_string());
            args.push(model.to_string());
        }
        if !provider.is_empty() {
            args.push("--provider".to_string());
            args.push(provider.to_string());
        }
        args.push("--oneshot".to_string());
        args.push(prompt.to_string());
        args
    }

    pub(super) fn spawn_hermes_exec(
        model: String,
        provider: String,
        prompt: String,
        tx: std::sync::mpsc::Sender<String>,
    ) {
        std::thread::spawn(move || {
            let args = Self::build_hermes_exec_args(&model, &provider, &prompt);
            match std::process::Command::new("hermes").args(&args).output() {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let response = if !stdout.trim().is_empty() {
                        stdout.trim().to_string()
                    } else if !stderr.trim().is_empty() {
                        format!("Error: {}", stderr.trim())
                    } else {
                        "(empty response)".to_string()
                    };
                    let _ = tx.send(response);
                }
                Err(e) => {
                    let _ = tx.send(format!("Failed to run Hermes Agent CLI: {e}"));
                }
            }
        });
    }
}
