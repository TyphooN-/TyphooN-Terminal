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

        self.write_symbol_investigation_question_and_return_path(&mut p, user_question);
        p
    }
}
