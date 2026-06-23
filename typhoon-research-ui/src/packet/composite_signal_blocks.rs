use super::context::SymbolResearchContext;
use crate::format;
use typhoon_engine::core::research as rx;

pub fn write_composite_signal_blocks(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    // Gather each composite snapshot and delegate to its pure formatter
    // (ADR-125 Phase 1 step 2); the emit guards live in the formatters.
    if let Ok(Some(gw)) = rx::get_growm(ctx.conn, &sym_upper) {
        format::write_growm(p, &gw);
    }
    if let Ok(Some(fl)) = rx::get_flow(ctx.conn, &sym_upper) {
        format::write_flow(p, &fl);
    }
    if let Ok(Some(rg)) = rx::get_regime(ctx.conn, &sym_upper) {
        format::write_regime(p, &rg);
    }
    if let Ok(Some(rv)) = rx::get_relvol(ctx.conn, &sym_upper) {
        format::write_relvol(p, &rv);
    }
    if let Ok(Some(mg)) = rx::get_margins(ctx.conn, &sym_upper) {
        format::write_margins(p, &mg);
    }
}
