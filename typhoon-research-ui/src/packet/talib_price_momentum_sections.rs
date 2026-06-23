use super::context::SymbolResearchContext;

pub fn write_symbol_talib_price_momentum_sections(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    super::talib_price_ohlc_stats::write_talib_price_ohlc_stats(ctx, p, sym_upper);
    super::talib_dmi_movement::write_talib_dmi_movement(ctx, p, sym_upper);
    super::talib_momentum_range::write_talib_momentum_range(ctx, p, sym_upper);
    super::talib_extended_emitters::write_talib_extended_emitters(ctx, p, sym_upper);
}
