use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_price_transform_indicator_sections(
        &self,
        p: &mut String,
        sym_upper: &str,
    ) {
        self.write_price_transform_adaptive_osc(p, sym_upper);

        self.write_price_transform_volatility_force(p, sym_upper);

        self.write_price_transform_linear_hilbert(p, sym_upper);

        self.write_price_transform_regression_phase(p, sym_upper);
    }
}
