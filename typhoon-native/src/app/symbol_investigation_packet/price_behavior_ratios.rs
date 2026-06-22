use super::context::SymbolResearchContext;
use super::format;
use typhoon_engine::core::research as rx;

pub(super) fn write_price_behavior_ratios(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    // Gather each snapshot from the research DB and hand it to the pure
    // formatter (ADR-125 Phase 1 step 2). The INSUFFICIENT_DATA guards
    // live inside the formatters.
    if let Ok(Some(sr)) = rx::get_sharpr(ctx.conn, &sym_upper) {
        format::write_sharpr(p, &sr);
    }
    if let Ok(Some(er)) = rx::get_effratio(ctx.conn, &sym_upper) {
        format::write_effratio(p, &er);
    }
    if let Ok(Some(wb)) = rx::get_wickbias(ctx.conn, &sym_upper) {
        format::write_wickbias(p, &wb);
    }
    if let Ok(Some(vv)) = rx::get_volofvol(ctx.conn, &sym_upper) {
        format::write_volofvol(p, &vv);
    }
}
