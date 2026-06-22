use super::context::SymbolResearchContext;

pub(super) fn write_symbol_composite_signal_sections(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    super::composite_signal_early::write_composite_signal_early(ctx, p, sym_upper);
    super::composite_signal_blocks::write_composite_signal_blocks(ctx, p, sym_upper);
    super::composite_signal_factors::write_composite_signal_factors(ctx, p, sym_upper);
}
