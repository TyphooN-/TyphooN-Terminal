use super::*;

impl TyphooNApp {
    pub(super) fn investigate_symbols(&self, syms: &[String], user_question: &str) -> String {
        use std::fmt::Write as _;
        let mut p = String::new();
        let _ = writeln!(p, "# TyphooN Terminal Research Packet");
        let _ = writeln!(
            p,
            "Scope: {} | Generated: {}",
            self.broker_scope_label(),
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
        );
        let _ = writeln!(p, "Symbols: {}", syms.join(", "));
        let _ = writeln!(p);

        self.write_symbol_investigation_global_context(&mut p);

        self.write_symbol_investigation_sections(&mut p, syms);

        self.write_imported_research_artifacts(&mut p, syms);

        self.write_symbol_investigation_question_and_return_path(&mut p, user_question);
        p
    }

    fn write_imported_research_artifacts(&self, p: &mut String, syms: &[String]) {
        use std::fmt::Write as _;
        let Some(cache) = self.cache.as_ref() else {
            return;
        };
        let mut artifacts = Vec::new();
        for sym in syms {
            let symbol = regulatory_alerts::normalize_regulatory_symbol(sym);
            let prefix = format!("research_artifact:{symbol}:");
            let Ok(mut keys) = cache.list_kv_keys(&prefix) else {
                continue;
            };
            keys.sort();
            for key in keys.into_iter().rev().take(5) {
                if let Ok(Some(json)) = cache.get_kv(&key) {
                    if let Ok(artifact) = serde_json::from_str::<ImportedResearchArtifact>(&json) {
                        artifacts.push(artifact);
                    }
                }
            }
        }
        if artifacts.is_empty() {
            return;
        }
        let _ = writeln!(p, "## Imported Research Artifacts");
        let _ = writeln!(p);
        for artifact in artifacts {
            let _ = writeln!(
                p,
                "### {} — {} ({})",
                artifact.symbol, artifact.filename, artifact.report_date
            );
            let _ = writeln!(p, "- Source file: `{}`", artifact.source_path);
            let _ = writeln!(p, "- Imported: {}", artifact.imported_at);
            let _ = writeln!(p);
            let _ = writeln!(p, "{}", artifact.content.trim());
            let _ = writeln!(p);
        }
    }
}
