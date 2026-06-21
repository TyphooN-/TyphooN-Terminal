use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_capital_valuation_sections(&self, p: &mut String, sym_upper: &str) {
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use super::format;
                use typhoon_engine::core::research as rx;

                // Each section gathers its snapshot from the research DB and hands
                // it to the pure formatter (ADR-125 Phase 1 step 2). The
                // per-snapshot emit guards (e.g. "only when WACC > 0") live inside
                // the formatters, so a missing/empty snapshot prints nothing.
                if let Ok(Some(w)) = rx::get_wacc(&conn, &sym_upper) {
                    format::write_wacc(p, &w);
                }
                if let Ok(Some(b)) = rx::get_beta(&conn, &sym_upper) {
                    format::write_beta(p, &b);
                }
                if let Ok(Some(d)) = rx::get_ddm(&conn, &sym_upper) {
                    format::write_ddm(p, &d);
                }
                if let Ok(Some(rv)) = rx::get_relative_valuation(&conn, &sym_upper) {
                    format::write_relative_valuation(p, &rv);
                }
                if let Ok(Some(f)) = rx::get_figi(&conn, &sym_upper) {
                    format::write_figi(p, &f);
                }
                if let Ok(Some(h)) = rx::get_hra(&conn, &sym_upper) {
                    format::write_hra(p, &h);
                }
                if let Ok(Some(d)) = rx::get_dcf(&conn, &sym_upper) {
                    format::write_dcf(p, &d);
                }
                if let Ok(Some(s)) = rx::get_svm(&conn, &sym_upper) {
                    format::write_svm(p, &s);
                }
                if let Ok(Some(o)) = rx::get_options_chain(&conn, &sym_upper) {
                    format::write_options_chain(p, &o);
                }
                if let Ok(Some(iv)) = rx::get_ivol(&conn, &sym_upper) {
                    format::write_ivol(p, &iv);
                }
            }
        }
    }
}
