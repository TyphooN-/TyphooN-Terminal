use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_template_workspace_command(&mut self, cmd_upper: &str) {
        match cmd_upper {
            other => {
                // Commands with arguments
                if other.starts_with("WORKSPACE_SAVE ") {
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
                            "Workspace '{}' not found (try TRADING/RESEARCH)",
                            name
                        )));
                    }
                } else if other == "WORKSPACES" {
                    let mut all: Vec<String> = self.workspaces.keys().cloned().collect();
                    all.extend(["TRADING (built-in)".into(), "RESEARCH (built-in)".into()]);
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
