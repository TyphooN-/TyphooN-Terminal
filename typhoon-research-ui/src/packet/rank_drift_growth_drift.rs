use super::context::SymbolResearchContext;
use crate::format;
use typhoon_engine::core::research as rx;

pub fn write_rank_drift_growth_drift(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    // Gather each rank/drift snapshot and delegate to its pure formatter
    // (ADR-125 Phase 1 step 2); the NO_DATA / INSUFFICIENT_DATA guards
    // live in the formatters.
    if let Ok(Some(eg)) = rx::get_relepsgr(ctx.conn, &sym_upper) {
        format::write_relepsgr(p, &eg);
    }
    if let Ok(Some(pd)) = rx::get_pead(ctx.conn, &sym_upper) {
        format::write_pead(p, &pd);
    }
    if let Ok(Some(sf)) = rx::get_sizef(ctx.conn, &sym_upper) {
        format::write_sizef(p, &sf);
    }
    if let Ok(Some(mf)) = rx::get_momf(ctx.conn, &sym_upper) {
        format::write_momf(p, &mf);
    }
}
