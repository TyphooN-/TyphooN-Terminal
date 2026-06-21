use super::*;

impl TyphooNApp {
    pub(super) fn write_rank_drift_growth_drift(&self, p: &mut String, sym_upper: &str) {
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use super::format;
                use typhoon_engine::core::research as rx;

                // Gather each rank/drift snapshot and delegate to its pure formatter
                // (ADR-125 Phase 1 step 2); the NO_DATA / INSUFFICIENT_DATA guards
                // live in the formatters.
                if let Ok(Some(eg)) = rx::get_relepsgr(&conn, &sym_upper) {
                    format::write_relepsgr(p, &eg);
                }
                if let Ok(Some(pd)) = rx::get_pead(&conn, &sym_upper) {
                    format::write_pead(p, &pd);
                }
                if let Ok(Some(sf)) = rx::get_sizef(&conn, &sym_upper) {
                    format::write_sizef(p, &sf);
                }
                if let Ok(Some(mf)) = rx::get_momf(&conn, &sym_upper) {
                    format::write_momf(p, &mf);
                }
            }
        }
    }
}
