use super::context::SymbolResearchContext;
use crate::format;
use typhoon_engine::core::research as rx;

/// Capital-valuation snapshots (WACC, Beta, DDM, RelVal, FIGI, HRA, DCF, SVM,
/// Options-chain, IVOL). Each is gathered from the research DB via the context's
/// shared connection and handed to the pure formatter. ADR-125 Phase 1 step 3:
/// a free function over the read-only context — no `TyphooNApp`, and it uses the
/// connection threaded from the dispatcher instead of re-acquiring `read_conn`.
/// The per-snapshot emit guards live in the formatters.
pub fn write_symbol_capital_valuation_sections(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    if let Ok(Some(w)) = rx::get_wacc(ctx.conn, sym_upper) {
        format::write_wacc(p, &w);
    }
    if let Ok(Some(b)) = rx::get_beta(ctx.conn, sym_upper) {
        format::write_beta(p, &b);
    }
    if let Ok(Some(d)) = rx::get_ddm(ctx.conn, sym_upper) {
        format::write_ddm(p, &d);
    }
    if let Ok(Some(rv)) = rx::get_relative_valuation(ctx.conn, sym_upper) {
        format::write_relative_valuation(p, &rv);
    }
    if let Ok(Some(f)) = rx::get_figi(ctx.conn, sym_upper) {
        format::write_figi(p, &f);
    }
    if let Ok(Some(h)) = rx::get_hra(ctx.conn, sym_upper) {
        format::write_hra(p, &h);
    }
    if let Ok(Some(d)) = rx::get_dcf(ctx.conn, sym_upper) {
        format::write_dcf(p, &d);
    }
    if let Ok(Some(s)) = rx::get_svm(ctx.conn, sym_upper) {
        format::write_svm(p, &s);
    }
    if let Ok(Some(o)) = rx::get_options_chain(ctx.conn, sym_upper) {
        format::write_options_chain(p, &o);
    }
    if let Ok(Some(iv)) = rx::get_ivol(ctx.conn, sym_upper) {
        format::write_ivol(p, &iv);
    }
}
