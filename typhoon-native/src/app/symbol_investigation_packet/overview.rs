use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_investigation_overview_sections(
        &self,
        p: &mut String,
        sym_upper: &str,
    ) {
        use std::fmt::Write as _;
        // User's open positions in this symbol — emit before fundamentals
        // so the AI treats the user's exposure as primary context when
        // answering questions like "what do you think about my position?".
        let pos_section = self.user_position_section(&sym_upper);
        if !pos_section.is_empty() {
            let _ = write!(p, "{pos_section}");
        }

        // Fundamentals row. Data-gathering (which record to use) stays here on
        // app state; the pure markdown formatting lives in
        // `format::write_fundamentals_overview` (ADR-125 Phase 1 step 2: section
        // formatters are free functions over engine DTOs so they stay
        // crate-movable).
        let fund = self
            .bg
            .all_fundamentals
            .iter()
            .find(|f| f.symbol.eq_ignore_ascii_case(&sym_upper));
        if let Some(f) = fund {
            super::format::write_fundamentals_overview(p, f);
        } else {
            let _ = writeln!(
                p,
                "_No fundamentals on file for this symbol. Run EVSCRAPE to populate._"
            );
            let _ = writeln!(p);
        }
    }
}
