use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_composite_signal_sections(&self, p: &mut String, sym_upper: &str) {
        self.write_composite_signal_early(p, sym_upper);

        self.write_composite_signal_blocks(p, sym_upper);

        self.write_composite_signal_factors(p, sym_upper);
    }
}
