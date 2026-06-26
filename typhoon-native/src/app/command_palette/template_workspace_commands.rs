use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_template_workspace_command(&mut self, cmd_upper: &str) {
        match cmd_upper {
            "TEMPLATES" | "LIST_TEMPLATES" => {
                let builtins = ["NNFX", "CLEAN", "FULL"];
                let builtins_set: std::collections::HashSet<&str> =
                    builtins.iter().copied().collect();
                let mut names: Vec<String> = builtins
                    .iter()
                    .map(|s| format!("{} (built-in)", s))
                    .collect();
                for k in self.chart_templates.keys() {
                    if !builtins_set.contains(&k.as_str()) {
                        names.push(k.clone());
                    }
                }
                names.sort();
                self.log
                    .push_back(LogEntry::info(format!("Templates: {}", names.join(", "))));
            }
            // ADR-092: UX improvement commands
            "COMPACT" => {
                self.compact_mode = !self.compact_mode;
                if self.compact_mode {
                    self.show_rsi = false;
                    self.show_fisher = false;
                    self.show_macd = false;
                    self.show_stochastic = false;
                    self.show_adx = false;
                    self.show_volume_pane = false;
                    self.show_better_volume = false;
                    self.log
                        .push_back(LogEntry::info("Compact mode ON — sub-panes hidden"));
                } else {
                    self.show_fisher = true;
                    self.show_better_volume = true;
                    self.log.push_back(LogEntry::info(
                        "Compact mode OFF — default indicators restored",
                    ));
                }
            }
            "RULER" => {
                self.log.push_back(LogEntry::info(
                    "Ruler: use trendline (Alt+T) to measure price/time distance",
                ));
            }
            other => {
                // Commands with arguments
                if other.starts_with("SAVE_TEMPLATE ") || other.starts_with("TEMPLATE_SAVE ") {
                    let name = other.splitn(2, ' ').nth(1).unwrap_or("").trim().to_string();
                    if name.is_empty() {
                        self.log
                            .push_back(LogEntry::warn("Usage: SAVE_TEMPLATE <name>"));
                    } else {
                        let snap = self.capture_indicator_snapshot();
                        self.chart_templates.insert(name.clone(), snap);
                        self.save_session();
                        self.log
                            .push_back(LogEntry::info(format!("Template '{}' saved", name)));
                    }
                } else if other.starts_with("LOAD_TEMPLATE ") || other.starts_with("TEMPLATE ") {
                    let name = other.splitn(2, ' ').nth(1).unwrap_or("").trim().to_string();
                    if name.is_empty() {
                        self.log
                            .push_back(LogEntry::warn("Usage: LOAD_TEMPLATE <name>"));
                    } else {
                        // Check built-in presets first
                        let template = match name.as_str() {
                            "NNFX" => Some(Self::builtin_template_nnfx()),
                            "CLEAN" => Some(Self::builtin_template_clean()),
                            "FULL" => Some(Self::builtin_template_full()),
                            _ => self.chart_templates.get(&name).cloned(),
                        };
                        if let Some(snap) = template {
                            self.apply_indicator_snapshot(&snap);
                            self.log
                                .push_back(LogEntry::info(format!("Template '{}' loaded", name)));
                        } else {
                            self.log.push_back(LogEntry::warn(format!(
                                "Template '{}' not found",
                                name
                            )));
                        }
                    }
                } else if other.starts_with("DELETE_TEMPLATE ") {
                    let name = other.splitn(2, ' ').nth(1).unwrap_or("").trim().to_string();
                    if self.chart_templates.remove(&name).is_some() {
                        self.save_session();
                        self.log
                            .push_back(LogEntry::info(format!("Template '{}' deleted", name)));
                    } else {
                        self.log
                            .push_back(LogEntry::warn(format!("Template '{}' not found", name)));
                    }
                } else if other.starts_with("WORKSPACE_SAVE ") {
                    let name = other.splitn(2, ' ').nth(1).unwrap_or("").trim().to_string();
                    if name.is_empty() {
                        self.log
                            .push_back(LogEntry::warn("Usage: WORKSPACE_SAVE <name>"));
                    } else {
                        let snap = self.capture_workspace_snapshot();
                        if let Ok(json) = serde_json::to_string(&snap) {
                            self.workspaces.insert(name.clone(), json);
                            self.save_session();
                            self.log
                                .push_back(LogEntry::info(format!("Workspace '{}' saved", name)));
                        }
                    }
                } else if other.starts_with("WORKSPACE_LOAD ") {
                    let name = other.splitn(2, ' ').nth(1).unwrap_or("").trim().to_string();
                    // Check user-saved first, then builtin presets
                    if let Some(json) = self.workspaces.get(&name).cloned() {
                        if let Ok(snap) = serde_json::from_str::<serde_json::Value>(&json) {
                            self.apply_workspace_snapshot(&snap);
                            self.log
                                .push_back(LogEntry::info(format!("Workspace '{}' loaded", name)));
                        }
                    } else if let Some(snap) = Self::builtin_workspace(&name) {
                        self.apply_workspace_snapshot(&snap);
                        self.log.push_back(LogEntry::info(format!(
                            "Built-in workspace '{}' loaded",
                            name.to_uppercase()
                        )));
                    } else {
                        self.log.push_back(LogEntry::warn(format!(
                            "Workspace '{}' not found (try TRADING/RESEARCH/COMPACT)",
                            name
                        )));
                    }
                } else if other == "WORKSPACES" {
                    let mut all: Vec<String> = self.workspaces.keys().cloned().collect();
                    all.extend([
                        "TRADING (built-in)".into(),
                        "RESEARCH (built-in)".into(),
                        "COMPACT (built-in)".into(),
                    ]);
                    self.log
                        .push_back(LogEntry::info(format!("Workspaces: {}", all.join(", "))));
                } else {
                    self.log
                        .push_back(LogEntry::warn(format!("Unknown command: {}", other)));
                }
            }
        }
    }
}
