use super::context::SymbolResearchContext;

pub fn write_symbol_price_transform_indicator_sections(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    super::price_transform_adaptive_osc::write_price_transform_adaptive_osc(ctx, p, sym_upper);
    super::price_transform_volatility_force::write_price_transform_volatility_force(
        ctx, p, sym_upper,
    );
    super::price_transform_linear_hilbert::write_price_transform_linear_hilbert(ctx, p, sym_upper);
    super::price_transform_regression_phase::write_price_transform_regression_phase(
        ctx, p, sym_upper,
    );
}
