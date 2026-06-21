use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_talib_price_momentum_sections(
        &self,
        p: &mut String,
        sym_upper: &str,
    ) {
        self.write_talib_price_ohlc_stats(p, sym_upper);

        self.write_talib_dmi_movement(p, sym_upper);

        self.write_talib_momentum_range(p, sym_upper);

        self.write_talib_extended_emitters(p, sym_upper);
    }
}
